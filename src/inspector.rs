use disqualified::ShortName;
use ratatui::{
    prelude::{BlockExt, Buffer, Rect},
    style::{Color, Stylize},
    text::{Line, Span},
    widgets::{Block, StatefulWidget, Widget},
};
use serde_json::{Map, Value};

use crate::PRIMARY_COLOR;

const INDENT_AMOUNT: u16 = 3;
const LINE_VERTICAL: &str = "│";
const LINE_JUNCTION: &str = "├";
const LINE_START: &str = "┌";
const LINE_END: &str = "└";

pub struct Inspector<'a> {
    value: &'a Value,
    block: Option<Block<'a>>,
    focused: bool,
}

impl<'a> Inspector<'a> {
    pub fn new(value: &'a Value, focused: bool) -> Self {
        Self {
            value,
            block: None,
            focused,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }

    fn fields(&self) -> usize {
        match self.value {
            Value::Object(obj) => obj.len(),
            _ => 1,
        }
    }
}

#[derive(Debug, Default)]
pub struct InspectorState {
    selected: usize,
}

impl InspectorState {
    pub fn select_previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn select_next(&mut self) {
        self.selected += 1;
    }
}

impl StatefulWidget for Inspector<'_> {
    type State = InspectorState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if let Some(block) = &self.block {
            block.render(area, buf);
        }
        let area = self.block.inner_if_some(area);

        if self.fields() == 0 {
            Line::raw("Nothing to show").bold().render(area, buf);
            return;
        }

        if let Value::Object(map) = self.value {
            let flat_map = flatten_value_map(map, 0);
            state.selected = state.selected.min(flat_map.len() - 1);
            for (n, field) in flat_map.iter().enumerate() {
                let short_name = ShortName(&field.name).to_string();
                let indent_chars = field.indent_level * INDENT_AMOUNT;
                let next_indent = flat_map.get(n + 1).map(|f| f.indent_level);

                let rect = Rect {
                    height: 1,
                    width: area.width,
                    x: area.x,
                    y: area.y + n as u16,
                };
                let label_rect = Rect {
                    width: indent_chars + short_name.len() as u16 + 5,
                    ..rect
                };
                let value_rect = Rect {
                    width: rect.width - label_rect.width,
                    x: rect.x + label_rect.width,
                    ..rect
                };

                let color = if n == state.selected && self.focused {
                    PRIMARY_COLOR
                } else {
                    Color::Reset
                };

                let label_line = Line::from(vec![
                    Span::raw(format!(
                        "{}{}─ ",
                        // LINE_VERTICAL.repeat(field.indent_level as usize),
                        (0..field.indent_level)
                            .map(|_| [LINE_VERTICAL, "  "])
                            .flatten()
                            .collect::<String>(),
                        match next_indent {
                            Some(level) if level < field.indent_level => LINE_END,
                            None => LINE_END,
                            _ if n == 0 => LINE_START,
                            _ => LINE_JUNCTION,
                        },
                    ))
                    .dim(),
                    Span::raw(&field.name).fg(color).bold(),
                    Span::raw(": ").fg(color),
                ]);
                label_line.render(label_rect, buf);

                InspectorValue(&field.value).render(value_rect, buf);
            }
        } else {
            state.selected = 0;
            InspectorValue(self.value).render(area, buf)
        }
    }
}

struct InspectorValue<'a>(&'a Value);

impl Widget for InspectorValue<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let line = match self.0 {
            Value::Null => Line::raw("None"),
            Value::Bool(value) => Line::raw(value.to_string()),
            Value::Number(value) => Line::raw(value.to_string()),
            Value::String(value) => Line::raw(value),
            Value::Array(_value) => Line::raw("Array (TODO)"),
            _ => panic!("Invalid value type"),
        };
        line.render(area, buf);
    }
}

struct Field {
    name: String,
    value: Value,
    indent_level: u16,
}

fn flatten_value_map(map: &Map<String, Value>, indent_level: u16) -> Vec<Field> {
    let mut vec = Vec::new();
    for (name, value) in map {
        match value {
            Value::Object(map) => vec.append(&mut flatten_value_map(map, indent_level + 1)),
            _ => vec.push(Field {
                name: name.to_owned(),
                value: value.to_owned(),
                indent_level,
            }),
        }
    }
    vec
}
