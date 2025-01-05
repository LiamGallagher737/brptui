use brp::BrpResponse;
use crossterm::event::{Event, KeyEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style, Stylize},
    symbols::border::THICK,
    text::{Line, Span, Text},
    widgets::{Block, Borders, Padding, Paragraph},
    DefaultTerminal, Frame,
};
use serde_json::Value;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::mpsc::{self, Receiver, Sender, TryRecvError},
    time::{Duration, Instant},
};
use ureq::json;

mod brp;
mod keys;

const PRIMARY_COLOR: Color = Color::Rgb(37, 160, 101);
const WHITE_COLOR: Color = Color::Rgb(255, 253, 245);

#[derive(argh::FromArgs)]
#[argh(description = "A tui for interacting with a Bevy application over BRP")]
#[argh(help_triggers("-h", "--help"))]
struct Args {
    /// the address to use for BRP requests.
    #[argh(
        option,
        short = 'i',
        default = "IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1))"
    )]
    ip: IpAddr,
    /// the port to use for BRP requests.
    #[argh(option, short = 'p', default = "15702")]
    port: u16,
    /// how many times per second to check for updates.
    #[argh(option, short = 'r', default = "10")]
    polling_rate: u8,
    /// print the version of brptui.
    #[argh(switch, short = 'v')]
    version: bool,
}

fn main() -> std::io::Result<()> {
    let args: Args = argh::from_env();

    if args.version {
        println!("{}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    let socket = SocketAddr::new(args.ip, args.port);
    let agent = ureq::agent();

    let (entities_sender, entities_receiver) = mpsc::channel();
    setup_query_thread(entities_sender, agent.clone(), socket, args.polling_rate);

    let app = App {
        socket,
        agent,
        polling_rate: args.polling_rate,
        exit: false,
        focus: Location::EntityList,
        entities: None,
        entities_receiver,
        entities_index: 0,
        components: None,
        components_receiver: None,
        components_index: 0,
    };

    let mut terminal = ratatui::init();
    let result = run(app, &mut terminal);
    ratatui::restore();
    result
}

#[derive(Debug)]
struct App {
    socket: SocketAddr,
    agent: ureq::Agent,
    polling_rate: u8,
    exit: bool,
    focus: Location,
    entities: Option<Vec<EntityMeta>>,
    entities_receiver: Receiver<Vec<EntityMeta>>,
    entities_index: usize,
    components: Option<Vec<(String, Value)>>,
    components_receiver: Option<Receiver<Vec<(String, Value)>>>,
    components_index: usize,
}

/// A focusable location in the app.
#[derive(Debug, PartialEq, Eq)]
enum Location {
    EntityList,
    ComponentList,
    ComponentInspector,
}

#[derive(Debug)]
struct EntityMeta {
    id: u64,
    name: Option<String>,
}

fn run(mut app: App, terminal: &mut DefaultTerminal) -> std::io::Result<()> {
    while !app.exit {
        terminal.draw(|frame| draw(&app, frame))?;
        handle_events(&mut app)?;

        match app.entities_receiver.try_recv() {
            Ok(entities) => {
                app.entities = Some(entities);
            }
            Err(TryRecvError::Disconnected) => {
                app.entities = None;
            }
            Err(TryRecvError::Empty) => {}
        }

        if let Some(receiver) = &app.components_receiver {
            match receiver.try_recv() {
                Ok(components) => {
                    app.components = Some(components);
                }
                Err(TryRecvError::Disconnected) => {
                    app.entities = None;
                }
                Err(TryRecvError::Empty) => {}
            }
        }
    }

    Ok(())
}

fn draw(app: &App, frame: &mut Frame) {
    let chunks = Layout::default()
        .constraints([
            Constraint::Length(1), // Header
            Constraint::Fill(1),   // Body
            Constraint::Length(1), // Footer
        ])
        .margin(1)
        .spacing(1)
        .split(frame.area());

    draw_header(app, frame, chunks[0]);
    draw_body(app, frame, chunks[1]);
    draw_footer(app, frame, chunks[2]);
}

fn draw_header(_app: &App, frame: &mut Frame, area: Rect) {
    let text = Text::styled(
        " brptui ",
        Style::default().fg(WHITE_COLOR).bg(PRIMARY_COLOR),
    );

    frame.render_widget(Paragraph::new(text), area);
}

fn draw_body(app: &App, frame: &mut Frame, area: Rect) {
    let horizontal_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Fill(1), // Entities list
            Constraint::Fill(1), // Components list
            Constraint::Fill(2), // Component inspector
        ])
        .spacing(1)
        .split(area);

    draw_body_entities_list(app, frame, horizontal_chunks[0]);
    draw_body_components_list(app, frame, horizontal_chunks[1]);
    draw_body_component_inspector(app, frame, horizontal_chunks[2]);
}

fn draw_body_entities_list(app: &App, frame: &mut Frame, area: Rect) {
    const LINES_PER_ENTRY: u16 = 1;

    let Some(entities) = &app.entities else {
        frame.render_widget(Paragraph::new(Text::raw("loading entities")), area);
        return;
    };

    let chunks = Layout::default()
        .constraints([
            Constraint::Fill(1),   // Entities list
            Constraint::Length(1), // Pagination dots
        ])
        .split(area);

    let list_area = chunks[0];
    let pagination_area = chunks[1];
    // Subtract 1 to leave gap before pagination without breaking border using Layout::spacing.
    let per_page = (list_area.height / LINES_PER_ENTRY) as usize - 1;
    let page = app.entities_index / per_page;
    let total_pages = entities.len().div_ceil(per_page);

    let page_entities =
        &entities[page * per_page..(page * per_page + per_page).min(entities.len())];

    let list_text: Vec<Line> = page_entities
        .iter()
        .enumerate()
        .map(|(n, entity_meta)| {
            let selected = n + (page * per_page) == app.entities_index;
            let name = entity_meta.name.clone().unwrap_or_else(|| {
                let entity = format!("{}v{}", entity_meta.id as u32, entity_meta.id >> 32);
                format!("Entity {entity}")
            });
            Line::styled(
                format!("{}{name}", if selected { "> " } else { "" }),
                if selected && app.focus == Location::EntityList {
                    Style::default().bold().fg(PRIMARY_COLOR)
                } else {
                    Style::default().bold()
                },
            )
        })
        .collect();

    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_set(THICK)
        .border_style(Style::default().fg(
            if matches!(app.focus, Location::EntityList | Location::ComponentList) {
                PRIMARY_COLOR
            } else {
                WHITE_COLOR
            },
        ));

    frame.render_widget(Paragraph::new(list_text).block(block.clone()), list_area);

    let pagination_text = Line::from(
        (0..total_pages)
            .map(|n| {
                if n != page {
                    Span::styled("• ", Style::default().dim())
                } else {
                    Span::raw("• ")
                }
            })
            .collect::<Vec<Span>>(),
    );

    frame.render_widget(
        Paragraph::new(pagination_text).block(block),
        pagination_area,
    );
}

fn draw_body_components_list(app: &App, frame: &mut Frame, area: Rect) {
    let Some(components) = &app.components else {
        return;
    };

    let list_text: Vec<_> = components
        .iter()
        .enumerate()
        .map(|(n, (name, _value))| {
            let selected = n == app.components_index;
            let short_name = disqualified::ShortName(&name);

            Line::styled(
                format!(
                    "{}{}",
                    if selected { "> " } else { "" },
                    short_name.to_string()
                ),
                if selected && app.focus == Location::ComponentList {
                    Style::default().bold().fg(PRIMARY_COLOR)
                } else {
                    Style::default().bold()
                },
            )
        })
        .collect();

    if list_text.is_empty() {
        frame.render_widget(
            Paragraph::new(Line::styled("Nothing to show", Style::default().bold())),
            area,
        );
        return;
    }

    frame.render_widget(Paragraph::new(list_text), area);
}

fn draw_body_component_inspector(app: &App, frame: &mut Frame, area: Rect) {
    let block = Block::default()
        .padding(Padding::left(1))
        .borders(Borders::LEFT)
        .border_set(THICK)
        .border_style(Style::default().fg(
            if matches!(
                app.focus,
                Location::ComponentList | Location::ComponentInspector
            ) {
                PRIMARY_COLOR
            } else {
                WHITE_COLOR
            },
        ));

    let Some(Some((_, component))) = app.components.as_ref().map(|c| c.get(app.components_index))
    else {
        frame.render_widget(Paragraph::new("").block(block), area);
        return;
    };

    let lines = render_value(component);
    if !lines.is_empty() {
        frame.render_widget(Paragraph::new(lines).block(block), area);
    } else {
        frame.render_widget(
            Paragraph::new(Line::styled("Nothing to show", Style::default().bold())).block(block),
            area,
        );
    }
}

fn render_value(value: &Value) -> Vec<Line> {
    match value {
        Value::Array(list) => {
            let mut lines = Vec::new();
            lines.push(Line::raw("["));
            lines.extend(
                list.iter()
                    .flat_map(render_value)
                    .map(indent_line)
                    .map(append_comma),
            );
            lines.push(Line::raw("]"));
            lines
        }
        Value::Object(map) => map
            .iter()
            .flat_map(|(name, value)| match value {
                Value::Object(_) => {
                    let mut lines = Vec::new();
                    lines.push(Line::from(vec![
                        Span::styled(format!("{name}: "), Style::default().bold()),
                        Span::raw("{"),
                    ]));
                    lines.extend(&mut render_value(value).drain(..).map(indent_line));
                    lines.push(Line::raw("},"));
                    lines
                }
                _ => {
                    let mut lines = render_value(value);
                    lines[0].spans.insert(
                        0,
                        Span::styled(format!("{name}: "), Style::default().bold()),
                    );
                    let len = lines.len();
                    lines[len - 1].spans.push(Span::raw(","));
                    lines
                }
            })
            .collect(),
        _ => vec![Line::raw(render_primitive_value(value))],
    }
}

fn render_primitive_value(value: &Value) -> String {
    match value {
        Value::Null => String::from("None"),
        Value::Bool(boolean) => boolean.to_string(),
        Value::Number(number) => number.to_string(),
        Value::String(string) => format!("{string:?}"),
        _ => panic!("Non-primitive value inputted"),
    }
}

fn draw_footer(app: &App, frame: &mut Frame, area: Rect) {
    let mut keys = Vec::new();
    keys.push(("s", "search"));
    if app.focus == Location::EntityList {
        keys.push(("x", "despawn"));
    }
    if app.focus == Location::ComponentList {
        keys.push(("x", "remove"));
    }
    keys.push(("q", "quit"));

    let text = Line::from(
        keys.iter()
            .enumerate()
            .flat_map(|(n, (key, description))| {
                [
                    Span::styled(*key, Style::default().dim().bold()),
                    Span::raw(" "),
                    Span::styled(*description, Style::default().dim()),
                    if n != keys.len() - 1 {
                        Span::styled(" • ", Style::default().dim())
                    } else {
                        Span::default()
                    },
                ]
            })
            .collect::<Vec<Span>>(),
    );

    frame.render_widget(Paragraph::new(text), area);
}

fn handle_events(app: &mut App) -> std::io::Result<()> {
    if !crossterm::event::poll(Duration::ZERO)? {
        return Ok(());
    }
    match crossterm::event::read()? {
        Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
            keys::handle_key_event(app, key_event)
        }
        _ => {}
    };

    Ok(())
}

fn setup_query_thread(
    sender: Sender<Vec<EntityMeta>>,
    agent: ureq::Agent,
    socket: SocketAddr,
    polling_rate: u8,
) {
    std::thread::spawn(move || {
        let duration = Duration::from_secs_f32(1.0 / polling_rate as f32);
        let mut last_time = Instant::now();
        loop {
            let Some(result) = brp_request(
                brp::BRP_QUERY_METHOD,
                json!({
                    "data": {
                        "option": ["bevy_core::name::Name"],
                    }
                }),
                agent.clone(),
                socket,
            ) else {
                return;
            };

            let rows = serde_json::from_value::<Vec<brp::BrpQueryRow>>(result)
                .expect("Failed to parse payload");

            let mut entities = rows
                .iter()
                .map(|row| EntityMeta {
                    id: row.entity,
                    name: row
                        .components
                        .get("bevy_core::name::Name")
                        .map(|name| name.get("name").unwrap().as_str().unwrap().to_string()),
                })
                .collect::<Vec<EntityMeta>>();

            entities.sort_by_key(|e| e.id);
            sender.send(entities).unwrap();

            // Sleep for the remaining time until the next poll.
            std::thread::sleep(duration.saturating_sub(last_time.elapsed()));
            last_time = Instant::now();
        }
    });
}

fn setup_get_thread(app: &mut App) {
    let Some(entities) = app.entities.as_ref() else {
        panic!("`App::entities` must be filled when seting up a get thread");
    };

    let entity = entities[app.entities_index].id;
    let polling_rate = app.polling_rate;
    let agent = app.agent.clone();
    let socket = app.socket;
    let (sender, receiver) = mpsc::channel();
    app.components_receiver = Some(receiver);
    app.components = None;

    std::thread::spawn(move || {
        let Some(result) = brp_request(
            brp::BRP_LIST_METHOD,
            json!({
                "entity": entity,
            }),
            agent.clone(),
            socket,
        ) else {
            return;
        };

        let Ok(components) = serde_json::from_value::<Vec<String>>(result) else {
            println!("Failed to parse payload");
            return;
        };

        let duration = Duration::from_secs_f32(1.0 / polling_rate as f32);
        let mut last_time = Instant::now();
        loop {
            let Some(result) = brp_request(
                brp::BRP_GET_METHOD,
                json!({
                    "entity": entity,
                    "components": components,
                }),
                agent.clone(),
                socket,
            ) else {
                return;
            };

            let Ok(response) = serde_json::from_value::<brp::BrpGetResponse>(result) else {
                println!("Failed to parse payload");
                return;
            };

            let mut components: Vec<(String, Value)> = response.components.into_iter().collect();
            components.sort_by(|(a, _), (b, _)| a.cmp(b));
            if sender.send(components).is_err() {
                return;
            }

            // Sleep for the remaining time until the next poll.
            std::thread::sleep(duration.saturating_sub(last_time.elapsed()));
            last_time = Instant::now();
        }
    });
}

fn brp_request(
    method: &str,
    params: Value,
    agent: ureq::Agent,
    socket: SocketAddr,
) -> Option<Value> {
    let result = agent
        .get(&format!("http://{socket}"))
        .send_json(brp::BrpRequest {
            jsonrpc: String::from("2.0"),
            method: String::from(method),
            id: None,
            params: Some(params),
        });

    let Ok(response) = result else {
        println!("Failed to send request");
        return None;
    };

    let Ok(body) = response.into_json::<BrpResponse>() else {
        println!("Failed to parse response");
        return None;
    };

    let brp::BrpPayload::Result(payload) = body.payload else {
        println!("Query failed");
        return None;
    };

    Some(payload)
}

fn indent_line(mut line: Line) -> Line {
    line.spans.insert(0, Span::raw("    "));
    line
}

fn append_comma(mut line: Line) -> Line {
    line.spans.push(Span::raw(","));
    line
}
