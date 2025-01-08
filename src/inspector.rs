use ratatui::{
    prelude::{BlockExt, Buffer, Rect},
    style::{Color, Stylize},
    text::{Line, Span},
    widgets::{Block, StatefulWidget, Widget},
};
use serde_json::{Map, Value};

use crate::PRIMARY_COLOR;

const INDENT_AMOUNT: u16 = 3;
const LINE_HORIZONTAL: &str = "─";
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
    pub fn new(value: &'a Value, focused: bool, state: &mut InspectorState) -> Self {
        state.value_types = match value {
            Value::Object(map) => flatten_value_map(map, 0)
                .iter()
                .filter_map(|line| match line {
                    InspecotorLine::ObjectStart { .. } => Some(ValueType::Object),
                    InspecotorLine::ObjectField { value, .. } => Some(ValueType::from(value)),
                    InspecotorLine::ArrayStart { value_type, .. } => Some(*value_type),
                    _ => None,
                })
                .collect(),
            _ => vec![ValueType::from(value)],
        };

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
    selected_array_item: Option<usize>,
    value_types: Vec<ValueType>,
    scroll: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueType {
    Null,
    Bool,
    Number,
    String,
    Array,
    Object,
}

impl InspectorState {
    pub fn select_previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn select_next(&mut self) {
        self.selected = (self.selected + 1).min(self.value_types.len() - 1);
    }

    pub fn selected_value_type(&self) -> ValueType {
        self.value_types[self.selected]
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
            let selected_y = flat_map
                .iter()
                .enumerate()
                .filter(|(_, l)| {
                    matches!(
                        l,
                        InspecotorLine::ObjectStart { .. }
                            | InspecotorLine::ArrayStart { .. }
                            | InspecotorLine::ObjectField { .. }
                    )
                })
                .nth(state.selected)
                .map(|(y, _)| y)
                .unwrap_or_default();
            if selected_y < state.scroll + 6 {
                state.scroll = state.scroll.saturating_sub(1);
            }
            if selected_y > state.scroll + area.height.saturating_sub(6) as usize {
                state.scroll += 1;
            }
            let upper_limit = (state.scroll + area.height as usize).min(flat_map.len());

            state.selected = state.selected.min(flat_map.len() - 1);
            let mut field_index = flat_map[0..state.scroll]
                .iter()
                .filter(|l| {
                    matches!(
                        l,
                        InspecotorLine::ObjectStart { .. }
                            | InspecotorLine::ArrayStart { .. }
                            | InspecotorLine::ObjectField { .. }
                    )
                })
                .count();
            for (y, line) in flat_map[state.scroll..upper_limit].iter().enumerate() {
                let rect = Rect {
                    height: 1,
                    width: area.width,
                    x: area.x,
                    y: area.y + y as u16,
                };

                let next_field = flat_map[field_index..].iter().find_map(|i| match i {
                    InspecotorLine::ObjectField { indent_level, .. } => Some(indent_level),
                    InspecotorLine::ArrayStart { indent_level, .. } => Some(indent_level),
                    _ => None,
                });

                match line {
                    InspecotorLine::ObjectField {
                        name,
                        value,
                        indent_level,
                    } => {
                        let indent_chars = indent_level * INDENT_AMOUNT;

                        let label_rect = Rect {
                            width: indent_chars + name.len() as u16 + 5,
                            ..rect
                        };
                        let value_rect = Rect {
                            width: rect.width - label_rect.width,
                            x: rect.x + label_rect.width,
                            ..rect
                        };

                        let color = if field_index == state.selected && self.focused {
                            PRIMARY_COLOR
                        } else {
                            Color::Reset
                        };

                        let label_line = Line::from(vec![
                            Span::raw(format!(
                                "{}{}{LINE_HORIZONTAL} ",
                                (0..*indent_level)
                                    .flat_map(|_| [LINE_VERTICAL, "  "])
                                    .collect::<String>(),
                                match flat_map.get(y + 1 + state.scroll) {
                                    _ if y == 0 => LINE_START,
                                    Some(InspecotorLine::ObjectField {
                                        indent_level: i, ..
                                    }) if indent_level == i => LINE_JUNCTION,
                                    Some(
                                        InspecotorLine::ObjectStart { .. }
                                        | InspecotorLine::ArrayStart { .. },
                                    ) => LINE_JUNCTION,
                                    Some(InspecotorLine::ObjectEnd { .. }) => LINE_END,
                                    None => LINE_END,
                                    _ => "X"
                                },
                            ))
                            .dim(),
                            Span::raw(name).fg(color).bold(),
                            Span::raw(": ").fg(color).bold(),
                        ]);
                        label_line.render(label_rect, buf);

                        InspectorValue(&value).render(value_rect, buf);
                        field_index += 1;
                    }
                    InspecotorLine::ArrayStart {
                        name, indent_level, ..
                    }
                    | InspecotorLine::ObjectStart { name, indent_level } => {
                        let color = if field_index == state.selected && self.focused {
                            PRIMARY_COLOR
                        } else {
                            Color::Reset
                        };

                        let c = match line {
                            InspecotorLine::ObjectStart { .. } => "{",
                            InspecotorLine::ArrayStart { .. } => "[",
                            _ => unreachable!(),
                        };

                        let label_line = Line::from(vec![
                            Span::raw(format!(
                                "{}{}{LINE_HORIZONTAL} ",
                                (0..*indent_level)
                                    .flat_map(|_| [LINE_VERTICAL, "  "])
                                    .collect::<String>(),
                                match next_field {
                                    _ if y == 0 => LINE_START,
                                    Some(level) if level < indent_level => LINE_END,
                                    None if y == 0 => LINE_HORIZONTAL,
                                    None => LINE_END,
                                    _ => LINE_JUNCTION,
                                },
                            ))
                            .dim(),
                            Span::raw(name).fg(color).bold(),
                            Span::raw(": ").fg(color).bold(),
                            Span::raw(c).bold(),
                        ]);
                        label_line.render(rect, buf);

                        field_index += 1;
                    }
                    InspecotorLine::ArrayItem {
                        value,
                        indent_level,
                    } => {
                        let label_line = Line::from(vec![
                            Span::raw(format!(
                                "{}   ",
                                (0..*indent_level + 1)
                                    .flat_map(|_| [LINE_VERTICAL, "  "])
                                    .collect::<String>(),
                            ))
                            .dim(),
                            Span::raw(value.to_string()),
                        ]);
                        label_line.render(rect, buf);
                    }
                    InspecotorLine::ArrayEnd { indent_level }
                    | InspecotorLine::ObjectEnd { indent_level } => {
                        let c = match line {
                            InspecotorLine::ObjectEnd { .. } => "}",
                            InspecotorLine::ArrayEnd { .. } => "]",
                            _ => unreachable!(),
                        };
                        let label_line = Line::from(vec![
                            Span::raw(format!(
                                "{}",
                                (0..*indent_level + 1)
                                    .flat_map(|_| [LINE_VERTICAL, "  "])
                                    .collect::<String>(),
                            ))
                            .dim(),
                            Span::raw(c).bold(),
                        ]);
                        label_line.render(rect, buf);
                    }
                }
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
            _ => panic!("Invalid value type"),
        };
        line.render(area, buf);
    }
}

#[derive(Debug)]
enum InspecotorLine {
    ObjectStart {
        name: String,
        indent_level: u16,
    },
    ObjectField {
        name: String,
        value: Value,
        indent_level: u16,
    },
    ObjectEnd {
        indent_level: u16,
    },
    ArrayStart {
        name: String,
        indent_level: u16,
        value_type: ValueType,
    },
    ArrayItem {
        value: Value,
        indent_level: u16,
    },
    ArrayEnd {
        indent_level: u16,
    },
}

fn flatten_value_map(map: &Map<String, Value>, indent_level: u16) -> Vec<InspecotorLine> {
    let mut vec = Vec::new();
    for (name, value) in map {
        match value {
            Value::Object(map) => {
                vec.push(InspecotorLine::ObjectStart {
                    name: name.to_owned(),
                    indent_level,
                });
                vec.append(&mut flatten_value_map(map, indent_level + 1));
                vec.push(InspecotorLine::ObjectEnd { indent_level });
            }
            Value::Array(items) => {
                vec.push(InspecotorLine::ArrayStart {
                    name: name.to_owned(),
                    indent_level,
                    value_type: ValueType::from(value),
                });
                for item in items {
                    vec.push(InspecotorLine::ArrayItem {
                        value: item.to_owned(),
                        indent_level,
                    });
                }
                vec.push(InspecotorLine::ArrayEnd { indent_level });
            }
            _ => vec.push(InspecotorLine::ObjectField {
                name: name.to_owned(),
                value: value.to_owned(),
                indent_level,
            }),
        }
    }
    vec
}

impl From<&Value> for ValueType {
    fn from(value: &Value) -> Self {
        match value {
            Value::Null => Self::Null,
            Value::Bool(_) => Self::Bool,
            Value::Number(_) => Self::Number,
            Value::String(_) => Self::String,
            Value::Array(_) => Self::Array,
            Value::Object(_) => Self::Object,
        }
    }
}
