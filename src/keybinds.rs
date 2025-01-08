use crate::{inspector::ValueType, Focus, State};
use ratatui::{
    prelude::{Buffer, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::Widget,
};

// Represents a single keybind
pub struct Keybind {
    pub keys: String,
    pub description: String,
    pub condition: KeybindCondition,
}

// Conditions under which a keybind is active
pub enum KeybindCondition {
    Always,
    Connected,
    Focus(Vec<Focus>),
    InspectorValue(Vec<ValueType>),
    Custom(Box<dyn Fn(&State) -> bool + Send>),
}

// Collection of keybinds with helper methods
#[derive(Default)]
pub struct KeybindSet {
    keybinds: Vec<Keybind>,
}

impl KeybindSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(
        &mut self,
        keys: impl Into<String>,
        description: impl Into<String>,
        condition: KeybindCondition,
    ) -> &mut Self {
        self.keybinds.push(Keybind {
            keys: keys.into(),
            description: description.into(),
            condition,
        });
        self
    }

    pub fn always(&mut self, keys: impl Into<String>, description: impl Into<String>) -> &mut Self {
        self.add(keys, description, KeybindCondition::Always)
    }

    pub fn when_connected(
        &mut self,
        keys: impl Into<String>,
        description: impl Into<String>,
    ) -> &mut Self {
        self.add(keys, description, KeybindCondition::Connected)
    }

    pub fn when_focus(
        &mut self,
        keys: impl Into<String>,
        description: impl Into<String>,
        focus: impl Into<Vec<Focus>>,
    ) -> &mut Self {
        self.add(keys, description, KeybindCondition::Focus(focus.into()))
    }

    pub fn when_inspector_value(
        &mut self,
        keys: impl Into<String>,
        description: impl Into<String>,
        value: impl Into<Vec<ValueType>>,
    ) -> &mut Self {
        self.add(
            keys,
            description,
            KeybindCondition::InspectorValue(value.into()),
        )
    }

    // Get active keybinds based on current state
    pub fn active_keybinds(&self, state: &State) -> Vec<(&str, &str)> {
        self.keybinds
            .iter()
            .filter(|kb| match &kb.condition {
                KeybindCondition::Always => true,
                KeybindCondition::Connected => matches!(state, State::Connected { .. }),
                KeybindCondition::Focus(required) => {
                    if let State::Connected { focus, .. } = state {
                        required.contains(focus)
                    } else {
                        false
                    }
                }
                KeybindCondition::InspectorValue(values) => {
                    if let State::Connected {
                        focus, inspector, ..
                    } = state
                    {
                        if *focus == Focus::Inspector {
                            return values.contains(&inspector.selected_value_type());
                        }
                    }
                    false
                }
                KeybindCondition::Custom(func) => func(state),
            })
            .map(|kb| (kb.keys.as_str(), kb.description.as_str()))
            .collect()
    }
}

// Widget to display active keybinds
pub struct KeybindDisplay<'a>(pub &'a [(&'a str, &'a str)]);

impl Widget for KeybindDisplay<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let keybinds_len = self.0.len();
        let text = Line::from(
            self.0
                .iter()
                .enumerate()
                .flat_map(|(n, (key, description))| {
                    let dim = Style::default().dim();
                    [
                        Span::styled(*key, dim.bold()),
                        Span::raw(" "),
                        Span::styled(*description, dim),
                        if n != keybinds_len - 1 {
                            Span::styled(" • ", dim)
                        } else {
                            Span::default()
                        },
                    ]
                })
                .collect::<Vec<Span>>(),
        );

        text.render(area, buf);
    }
}
