use bevy_remote::builtin_methods::BrpDestroyParams;
use brp::{handle_components_querying, EntityMeta};
use disqualified::ShortName;
use keybinds::Keybinds;
use paginated_list::{PaginatedList, PaginatedListState};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{palette::material::WHITE, Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Padding, Paragraph},
    Frame,
};
use serde_json::Value;
use std::{net::SocketAddr, sync::mpsc, thread};

mod brp;
mod events;
mod keybinds;
mod paginated_list;

const PRIMARY_COLOR: Color = Color::Rgb(37, 160, 101);

#[derive(Debug)]
struct Model {
    state: State,
    focus: Focus,
    socket: SocketAddr,
    message_tx: mpsc::Sender<Message>,
}

impl Model {
    fn new(message_tx: mpsc::Sender<Message>) -> Self {
        Self {
            state: Default::default(),
            focus: Default::default(),
            socket: brp::DEFAULT_SOCKET,
            message_tx,
        }
    }
}

#[derive(Debug, Default)]
enum State {
    Connected {
        entities: Vec<EntityMeta>,
        entities_list: PaginatedListState,
        components: Vec<(String, Value)>,
        components_list: PaginatedListState,
    },
    #[default]
    Disconnected,
    Done,
}

#[derive(Debug)]
enum Message {
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    PageUp,
    PageDown,
    Delete,
    SpawnComponnentsThread,
    UpdateEntities(Vec<EntityMeta>),
    UpdateComponents(Vec<(String, Value)>),
    CommunicationFailed,
    Quit,
}

/// Areas that a user can focus on.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Focus {
    /// The panel listing all entities in the world.
    #[default]
    Entities,
    /// The panel listing all (reflectable) components on the selected entity.
    Components,
    /// The panel displaying the value of the selected component.
    Inspector,
    /// The searchbar
    Search,
}

fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();

    let (tx, rx) = mpsc::channel();
    let mut model = Model::new(tx.clone());

    // Spawn crossterm event handler thread.
    let events_tx = tx.clone();
    thread::spawn(move || events::handle_events(events_tx));

    // Spawn BRP entity querying thread.
    let querying_tx = tx.clone();
    thread::spawn(move || brp::handle_entity_querying(querying_tx, &model.socket));

    // Panic rather than return `Err` within loop to ensure terminal is restored.
    // TODO: Improve this so an error can be returned.
    while !matches!(model.state, State::Done) {
        // Render the current view
        terminal.draw(|f| view(&mut model, f)).unwrap();

        // Wait for next external message.
        let mut next_msg = Some(rx.recv().unwrap());

        // Process updates as long as they return a non-None message.
        while let Some(msg) = next_msg {
            next_msg = update(&mut model, msg);
        }
    }

    ratatui::restore();
    Ok(())
}

fn view(model: &mut Model, frame: &mut Frame) {
    let layout = Layout::default()
        .constraints([
            Constraint::Length(1), // Header
            Constraint::Fill(1),   // Body
            Constraint::Length(1), // Footer
        ])
        .margin(1)
        .spacing(1)
        .split(frame.area());

    // Header
    let text = Text::styled(" brptui ", Style::default().fg(WHITE).bg(PRIMARY_COLOR));
    frame.render_widget(Paragraph::new(text), layout[0]);

    // Body
    match &mut model.state {
        State::Connected {
            entities,
            entities_list,
            components,
            components_list,
        } => {
            let body_layout = Layout::new(
                Direction::Horizontal,
                [
                    Constraint::Fill(1),
                    Constraint::Fill(1),
                    Constraint::Fill(2),
                ],
            )
            .split(layout[1]);

            let entities_block = Block::default()
                .borders(Borders::RIGHT)
                .border_type(BorderType::Thick)
                .border_style(border_style(matches!(
                    model.focus,
                    Focus::Entities | Focus::Components
                )));

            frame.render_stateful_widget(
                PaginatedList::new(
                    entities.iter().map(EntityMeta::title),
                    model.focus == Focus::Entities,
                )
                .block(entities_block),
                body_layout[0],
                entities_list,
            );

            frame.render_stateful_widget(
                PaginatedList::new(
                    components
                        .iter()
                        .map(|(name, _)| ShortName(name).to_string())
                        .map(Span::raw)
                        .map(Span::bold)
                        .map(Line::from),
                    model.focus == Focus::Components,
                )
                .block(Block::default().padding(Padding::horizontal(1))),
                body_layout[1],
                components_list,
            );
        }
        State::Disconnected => {
            frame.render_widget(Paragraph::new("Disconnected"), layout[1]);
        }
        State::Done => {}
    }

    // Footer
    let mut keys = Keybinds::default();
    keys.push("s", "search");
    if model.focus == Focus::Entities {
        keys.push("x", "despawn");
    }
    if model.focus == Focus::Components {
        keys.push("x", "remove");
    }
    keys.push("q", "quit");
    frame.render_widget(keys, layout[2]);
}

fn update(model: &mut Model, msg: Message) -> Option<Message> {
    // Will be able to improve with https://github.com/rust-lang/rust/issues/51114
    match msg {
        Message::MoveLeft => match model.focus {
            Focus::Components => model.focus = Focus::Entities,
            Focus::Inspector => model.focus = Focus::Components,
            _ => {}
        },
        Message::MoveRight => match model.focus {
            Focus::Entities => model.focus = Focus::Components,
            Focus::Components => model.focus = Focus::Inspector,
            _ => {}
        },
        Message::MoveUp => match &mut model.state {
            State::Connected {
                entities_list,
                components_list,
                ..
            } => match model.focus {
                Focus::Entities => entities_list.select_previous(),
                Focus::Components => components_list.select_previous(),
                _ => {}
            },
            _ => {}
        },
        Message::MoveDown => match &mut model.state {
            State::Connected {
                entities_list,
                components_list,
                ..
            } => match model.focus {
                Focus::Entities => entities_list.select_next(),
                Focus::Components => components_list.select_next(),
                _ => {}
            },
            _ => {}
        },
        Message::PageUp => match &mut model.state {
            State::Connected {
                entities_list,
                components_list,
                ..
            } => match model.focus {
                Focus::Entities => entities_list.select_previous_page(),
                Focus::Components => components_list.select_previous_page(),
                _ => {}
            },
            _ => {}
        },
        Message::PageDown => match &mut model.state {
            State::Connected {
                entities_list,
                components_list,
                ..
            } => match model.focus {
                Focus::Entities => entities_list.select_next_page(),
                Focus::Components => components_list.select_next_page(),
                _ => {}
            },
            _ => {}
        },
        Message::Delete => {
            if let State::Connected {
                entities,
                entities_list,
                ..
            } = &mut model.state
            {
                let socket = model.socket;
                match model.focus {
                    Focus::Entities => {
                        let entity = entities.remove(entities_list.selected()).id;
                        thread::spawn(move || {
                            let _ = brp::destroy_request(&socket, BrpDestroyParams { entity });
                        });
                    }
                    Focus::Components => todo!(),
                    _ => {}
                }
            }
        }
        Message::SpawnComponnentsThread => {
            if let State::Connected {
                entities,
                entities_list,
                ..
            } = &model.state
            {
                let tx = model.message_tx.clone();
                let socket = model.socket;
                let entity = entities[entities_list.selected()].id;
                thread::spawn(move || handle_components_querying(tx, &socket, entity));
            }
        }
        Message::UpdateEntities(new_entities) => match &mut model.state {
            State::Connected { entities, .. } => *entities = new_entities,
            _ => {
                model.state = State::Connected {
                    entities: new_entities,
                    entities_list: PaginatedListState::default(),
                    components: Vec::new(),
                    components_list: PaginatedListState::default(),
                };
                return Some(Message::SpawnComponnentsThread);
            }
        },
        Message::UpdateComponents(new_components) => {
            if let State::Connected { components, .. } = &mut model.state {
                *components = new_components;
            }
        }
        Message::CommunicationFailed => {
            model.state = State::Disconnected;
        }
        Message::Quit => {
            model.state = State::Done;
        }
    };

    None
}

fn border_style(focused: bool) -> Style {
    if focused {
        Style::default().fg(PRIMARY_COLOR)
    } else {
        Style::default().dim()
    }
}
