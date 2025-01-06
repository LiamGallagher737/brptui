use ratatui::{
    layout::{Constraint, Layout},
    style::{Color, Style},
    text::{Line, Text},
    widgets::Paragraph,
    Frame,
};
use std::{net::SocketAddr, sync::mpsc, thread};

mod brp;
mod events;

const PRIMARY_COLOR: Color = Color::Rgb(37, 160, 101);
const WHITE_COLOR: Color = Color::Rgb(255, 253, 245);

#[derive(Debug)]
struct Model {
    state: State,
    focus: Focus,
    selected_indicies: [usize; 3],
    max_indicies: [usize; 3],
    socket: SocketAddr,
}

impl Default for Model {
    fn default() -> Self {
        Self {
            state: Default::default(),
            focus: Default::default(),
            selected_indicies: [0; 3],
            max_indicies: [0; 3],
            socket: brp::DEFAULT_SOCKET,
        }
    }
}

#[derive(Debug, Default)]
enum State {
    Connected {
        entities: Vec<brp::EntityMeta>,
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
    UpdateEntities(Vec<brp::EntityMeta>),
    CommunicationFailed,
    Quit,
}

/// Areas that a user can focus on.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
enum Focus {
    /// The panel listing all entities in the world.
    #[default]
    Entitties,
    /// The panel listing all (reflectable) components on the selected entity.
    Components,
    /// The panel displaying the value of the selected component.
    Inspector,
    /// The searchbar
    Search,
}

fn main() -> std::io::Result<()> {
    let mut terminal = ratatui::init();
    let mut model = Model::default();

    // Setup a mpsc channel for messages to be sent from multiple threads.
    let (tx, rx) = mpsc::channel();

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
        terminal.draw(|f| view(&model, f)).unwrap();

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

fn view(model: &Model, frame: &mut Frame) {
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
    let text = Text::styled(
        " brptui ",
        Style::default().fg(WHITE_COLOR).bg(PRIMARY_COLOR),
    );
    frame.render_widget(Paragraph::new(text), layout[0]);

    // Body
    match &model.state {
        State::Connected { entities } => {
            frame.render_widget(
                Paragraph::new(
                    entities
                        .iter()
                        .map(|entity| {
                            Line::raw(entity.name.clone().unwrap_or_else(|| entity.id.to_string()))
                        })
                        .collect::<Vec<_>>(),
                ),
                layout[1],
            );
        }
        State::Disconnected => {
            frame.render_widget(Paragraph::new("Disconnected"), layout[1]);
        }
        State::Done => {}
    }
}

fn update(model: &mut Model, msg: Message) -> Option<Message> {
    match msg {
        Message::MoveLeft => match model.focus {
            Focus::Components => model.focus = Focus::Entitties,
            Focus::Inspector => model.focus = Focus::Components,
            _ => {}
        },
        Message::MoveRight => match model.focus {
            Focus::Entitties => model.focus = Focus::Components,
            Focus::Components => model.focus = Focus::Inspector,
            _ => {}
        },
        Message::MoveUp => {
            let focus = model.focus as usize;
            if model.selected_indicies[focus] > 0 {
                model.selected_indicies[focus] -= 1;
            } else {
                model.selected_indicies[focus] = model.max_indicies[focus] - 1;
            }
        }
        Message::MoveDown => {
            let focus = model.focus as usize;
            if model.selected_indicies[focus] < model.max_indicies[focus] - 1 {
                model.selected_indicies[focus] += 1;
            } else {
                model.selected_indicies[focus] = 0;
            }
        }
        Message::UpdateEntities(entities) => {
            model.max_indicies[Focus::Entitties as usize] = entities.len();
            model.state = State::Connected { entities };
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
