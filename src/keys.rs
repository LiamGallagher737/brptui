use crate::{setup_get_thread, App, Location};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub fn handle_key_event(app: &mut App, key_event: KeyEvent) {
    let entities_len = app.entities.as_ref().map(|e| e.len()).unwrap_or_default();
    let components_len = app.components.as_ref().map(|e| e.len()).unwrap_or_default();

    match key_event.code {
        // Quitting
        KeyCode::Char('q') => app.exit = true,
        KeyCode::Char('c') if key_event.modifiers.contains(KeyModifiers::CONTROL) => {
            app.exit = true
        }

        // Entity list
        KeyCode::Up | KeyCode::Char('k')
            if app.focus == Location::EntityList && app.entities.is_some() =>
        {
            if app.entities_index > 0 {
                app.entities_index -= 1;
            } else {
                app.entities_index = entities_len - 1;
            };
            app.components_index = 0;
            setup_get_thread(app);
        }
        KeyCode::Down | KeyCode::Char('j')
            if app.focus == Location::EntityList && app.entities.is_some() =>
        {
            if app.entities_index < entities_len - 1 {
                app.entities_index += 1;
            } else {
                app.entities_index = 0;
            };
            app.components_index = 0;
            setup_get_thread(app);
        }

        // Component list
        KeyCode::Up | KeyCode::Char('k')
            if app.focus == Location::ComponentList && app.components.is_some() =>
        {
            if app.components_index > 0 {
                app.components_index -= 1;
            } else {
                app.components_index = components_len.saturating_sub(1);
            };
        }
        KeyCode::Down | KeyCode::Char('j')
            if app.focus == Location::ComponentList && app.components.is_some() =>
        {
            if app.components_index < components_len.saturating_sub(1) {
                app.components_index += 1;
            } else {
                app.components_index = 0;
            };
        }

        // Panel switching
        KeyCode::Left | KeyCode::Char('h') => {
            app.focus = match app.focus {
                Location::EntityList => return,
                Location::ComponentList => Location::EntityList,
                Location::ComponentInspector => Location::ComponentList,
            };
        }
        KeyCode::Right | KeyCode::Char('l') => {
            app.focus = match app.focus {
                Location::EntityList => Location::ComponentList,
                Location::ComponentList => Location::ComponentInspector,
                Location::ComponentInspector => return,
            };
        }
        KeyCode::Tab => {
            app.focus = match app.focus {
                Location::EntityList => Location::ComponentList,
                Location::ComponentList => Location::ComponentInspector,
                Location::ComponentInspector => Location::EntityList,
            };
        }
        KeyCode::BackTab => {
            app.focus = match app.focus {
                Location::EntityList => Location::ComponentInspector,
                Location::ComponentList => Location::EntityList,
                Location::ComponentInspector => Location::ComponentList,
            };
        }

        _ => {}
    }
}
