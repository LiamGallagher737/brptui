use bevy_remote::builtin_methods::{BrpDestroyParams, BrpRemoveParams};
use brp::{handle_components_querying, EntityMeta};
use disqualified::ShortName;
use inspector::{Inspector, InspectorState, ValueType};
use keybinds::{KeybindDisplay, KeybindSet};
use paginated_list::{PaginatedList, PaginatedListState};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{palette::material::WHITE, Color, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{Block, BorderType, Borders, Padding, Paragraph},
    Frame,
};
use serde_json::Value;
use std::{
    net::SocketAddr,
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc,
    },
    thread,
};

mod brp;
mod events;
mod inspector;
mod keybinds;
mod paginated_list;

const PRIMARY_COLOR: Color = Color::Rgb(37, 160, 101);

struct Model {
    state: State,
    socket: SocketAddr,
    message_tx: mpsc::Sender<Message>,
    keybinds: KeybindSet,
}

impl Model {
    fn new(message_tx: mpsc::Sender<Message>, keybinds: KeybindSet) -> Self {
        Self {
            state: Default::default(),
            socket: brp::DEFAULT_SOCKET,
            message_tx,
            keybinds,
        }
    }
}

#[derive(Debug, Default)]
enum State {
    Connected {
        focus: Focus,
        entities: Vec<EntityMeta>,
        entities_list: PaginatedListState,
        components: Vec<(String, Value)>,
        components_list: PaginatedListState,
        components_thread_quitter: Option<ThreadQuitToken>,
        inspector: InspectorState,
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
    Home,
    End,
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

    // Keybinds will be displayed in the order they are added
    let mut keybinds = KeybindSet::new();
    keybinds
        .always("s", "search")
        .when_focus("x", "despawn", [Focus::Entities])
        .when_focus("x", "remove", [Focus::Components])
        .when_focus("[]", "move page", [Focus::Entities, Focus::Components])
        .when_inspector_value("t", "toggle", [ValueType::Bool])
        .when_inspector_value("e", "edit", [ValueType::Number, ValueType::String])
        .when_connected("hjkl/←↓↑→", "move")
        .always("q", "quit");

    let (tx, rx) = mpsc::channel();
    let mut model = Model::new(tx.clone(), keybinds);

    // Spawn crossterm event handler thread.
    let events_tx = tx.clone();
    thread::spawn(move || events::handle_events(events_tx));

    // Spawn BRP entity querying thread.
    let querying_tx = tx.clone();
    thread::spawn(move || brp::handle_entity_querying(querying_tx, &model.socket));

    while !matches!(model.state, State::Done) {
        let mut next_msg = Some(rx.recv().unwrap());

        // Process updates as long as they return a non-None message.
        // Render after every update so stateful widgets can update their state.
        while let Some(msg) = next_msg {
            next_msg = update(&mut model, msg);
            terminal.draw(|f| view(&mut model, f))?;
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
            focus,
            entities,
            entities_list,
            components,
            components_list,
            inspector,
            ..
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
                    focus,
                    Focus::Entities | Focus::Components
                )));

            let components_block = Block::default().padding(Padding::horizontal(1));

            let inspector_block = Block::default()
                .padding(Padding::left(1))
                .borders(Borders::LEFT)
                .border_type(BorderType::Thick)
                .border_style(border_style(matches!(
                    focus,
                    Focus::Components | Focus::Inspector
                )));

            frame.render_stateful_widget(
                PaginatedList::new(
                    entities.iter().map(EntityMeta::title),
                    *focus == Focus::Entities,
                )
                .block(entities_block),
                body_layout[0],
                entities_list,
            );

            if !components.is_empty() {
                frame.render_stateful_widget(
                    PaginatedList::new(
                        components
                            .iter()
                            .map(|(name, _)| ShortName(name).to_string())
                            .map(Span::raw)
                            .map(Span::bold)
                            .map(Line::from),
                        *focus == Focus::Components,
                    )
                    .block(components_block),
                    body_layout[1],
                    components_list,
                );
            } else {
                frame.render_widget(
                    Paragraph::new("Nothing to show")
                        .bold()
                        .block(components_block),
                    body_layout[1],
                );
            }

            if let Some(selected_component) = components.get(components_list.selected()) {
                frame.render_stateful_widget(
                    Inspector::new(&selected_component.1, *focus == Focus::Inspector)
                        .block(inspector_block),
                    body_layout[2],
                    inspector,
                );
            }
        }
        State::Disconnected => {
            frame.render_widget(Paragraph::new("Disconnected"), layout[1]);
        }
        State::Done => {}
    }

    // Footer
    let active_keybinds = model.keybinds.active_keybinds(&model.state);
    frame.render_widget(KeybindDisplay(&active_keybinds[..]), layout[2]);
}

macro_rules! handle_movement {
    ($msg:expr, $state:expr, {
        $($focus_pattern:pat => $list:ident $method:ident $(=> $after:expr)?),* $(,)?
    }) => {
        if let State::Connected { focus, $($list,)* .. } = $state {
            match focus {
                $(
                    $focus_pattern => {
                        $list.$method();
                        $(return Some($after);)?
                    }
                )*
                _ => {}
            }
        }
    };
}

fn update(model: &mut Model, msg: Message) -> Option<Message> {
    match (msg, &mut model.state) {
        // Navigation between panels
        (Message::MoveLeft, State::Connected { focus, .. }) => {
            *focus = match *focus {
                Focus::Components => Focus::Entities,
                Focus::Inspector => Focus::Components,
                _ => *focus,
            };
        }

        (
            Message::MoveRight,
            State::Connected {
                focus, components, ..
            },
        ) => {
            *focus = match *focus {
                Focus::Entities if !components.is_empty() => Focus::Components,
                Focus::Components => Focus::Inspector,
                _ => *focus,
            };
        }

        (Message::MoveLeft | Message::MoveRight, _) => {}

        // Movement within panels
        (Message::MoveUp, state) => {
            handle_movement!(Message::MoveUp, state, {
                Focus::Entities => entities_list select_previous => Message::SpawnComponnentsThread,
                Focus::Components => components_list select_previous,
                Focus::Inspector => inspector select_previous,
            });
        }

        (Message::MoveDown, state) => {
            handle_movement!(Message::MoveDown, state, {
                Focus::Entities => entities_list select_next => Message::SpawnComponnentsThread,
                Focus::Components => components_list select_next,
                Focus::Inspector => inspector select_next,
            });
        }

        (Message::PageUp, state) => {
            handle_movement!(Message::PageUp, state, {
                Focus::Entities => entities_list select_previous_page => Message::SpawnComponnentsThread,
                Focus::Components => components_list select_previous_page,
            });
        }

        (Message::PageDown, state) => {
            handle_movement!(Message::PageDown, state, {
                Focus::Entities => entities_list select_next_page => Message::SpawnComponnentsThread,
                Focus::Components => components_list select_next_page,
            });
        }

        (Message::Home, state) => {
            handle_movement!(Message::Home, state, {
                Focus::Entities => entities_list select_first,
                Focus::Components => components_list select_first,
                Focus::Inspector => inspector select_first,
            });
        }

        (Message::End, state) => {
            handle_movement!(Message::Home, state, {
                Focus::Entities => entities_list select_last,
                Focus::Components => components_list select_last,
                Focus::Inspector => inspector select_last,
            });
        }

        // Deletion operations
        (
            Message::Delete,
            State::Connected {
                focus,
                entities,
                entities_list,
                components,
                components_list,
                ..
            },
        ) => {
            let socket = model.socket;
            match focus {
                Focus::Entities => {
                    let entity = entities.remove(entities_list.selected()).id;
                    thread::spawn(move || {
                        let _ = brp::destroy_request(&socket, BrpDestroyParams { entity });
                    });
                }
                Focus::Components => {
                    let entity = entities[entities_list.selected()].id;
                    let (component, _) = components.remove(components_list.selected());
                    thread::spawn(move || {
                        let _ = brp::remove_request(
                            &socket,
                            BrpRemoveParams {
                                entity,
                                components: vec![component.to_owned()],
                            },
                        );
                    });
                }
                _ => {}
            }
        }
        (Message::Delete, _) => {}

        // Thread management
        (
            Message::SpawnComponnentsThread,
            State::Connected {
                entities,
                entities_list,
                components_thread_quitter,
                ..
            },
        ) => {
            if let Some(quitter) = components_thread_quitter {
                quitter.quit();
            }
            let tx = model.message_tx.clone();
            let socket = model.socket;
            let entity = entities[entities_list.selected()].id;
            let quitter = ThreadQuitToken::new();
            *components_thread_quitter = Some(quitter.clone());
            thread::spawn(move || handle_components_querying(tx, &socket, entity, quitter));
        }
        (Message::SpawnComponnentsThread, _) => {}

        // State updates
        (Message::UpdateEntities(new_entities), State::Connected { entities, .. }) => {
            *entities = new_entities;
        }
        (Message::UpdateEntities(new_entities), _) => {
            model.state = State::Connected {
                focus: Focus::default(),
                entities: new_entities,
                entities_list: PaginatedListState::default(),
                components: Vec::new(),
                components_list: PaginatedListState::default(),
                components_thread_quitter: None,
                inspector: InspectorState::default(),
            };
            return Some(Message::SpawnComponnentsThread);
        }

        (Message::UpdateComponents(new_components), State::Connected { components, .. }) => {
            *components = new_components;
        }
        (Message::UpdateComponents(_), _) => {}

        // State transitions
        (Message::CommunicationFailed, _) => {
            model.state = State::Disconnected;
        }
        (Message::Quit, _) => {
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

#[derive(Debug, Default, Clone)]
struct ThreadQuitToken {
    quit: Arc<AtomicBool>,
}

impl ThreadQuitToken {
    fn new() -> Self {
        Self {
            quit: Arc::new(AtomicBool::new(false)),
        }
    }

    fn quit(&mut self) {
        self.quit.store(true, Ordering::Relaxed);
    }

    fn should_quit(&self) -> bool {
        self.quit.load(Ordering::Relaxed)
    }
}
