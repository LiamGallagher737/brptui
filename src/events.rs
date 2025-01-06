//! Logic for handling [`event::Event`]s.

use crate::Message;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind};
use std::sync::mpsc;

/// Resulting [`Message`]s will be sent using the given [`mpsc::Sender`] to the
/// main thread to be handled.
pub fn handle_events(tx: mpsc::Sender<Message>) {
    loop {
        let message = match event::read().unwrap() {
            Event::Key(key) if key.kind == KeyEventKind::Press => handle_key(key),
            _ => None,
        };

        if let Some(msg) = message {
            tx.send(msg).unwrap();
        }
    }
}

fn handle_key(key: event::KeyEvent) -> Option<Message> {
    match key.code {
        KeyCode::Left | KeyCode::Char('h') => Some(Message::MoveLeft),
        KeyCode::Right | KeyCode::Char('l') => Some(Message::MoveRight),
        KeyCode::Up | KeyCode::Char('k') => Some(Message::MoveUp),
        KeyCode::Down | KeyCode::Char('j') => Some(Message::MoveDown),
        KeyCode::PageUp | KeyCode::Char('[') => Some(Message::PageUp),
        KeyCode::PageDown | KeyCode::Char(']') => Some(Message::PageDown),
        KeyCode::Char('q') => Some(Message::Quit),
        _ => None,
    }
}
