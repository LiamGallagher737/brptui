use crate::PRIMARY_COLOR;
use ratatui::{
    prelude::{BlockExt, Buffer, Rect},
    style::{Color, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, StatefulWidget, Widget},
};
use serde_json::{Number, Value};

const INDENT_AMOUNT: u16 = 3;

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
    paths: Vec<String>,
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

        let flat_map = flatten_value(self.value);

        state.update_paths(&flat_map);
        state.update_value_types(&flat_map);
        state.update_selected(&flat_map);
        state.update_scroll(&flat_map, area.height);
        let upper_limit = (state.scroll + area.height as usize).min(flat_map.len());

        for (y, line) in flat_map[state.scroll..upper_limit].iter().enumerate() {
            let mut rect = Rect {
                height: 1,
                width: area.width,
                x: area.x,
                y: area.y + y as u16,
            };

            let selected = self.focused && line.path == state.selected_path();

            // Since the indent is just blank space there is no point rendering anything and the
            // space can just be subtracted from the lines rect.
            let _indent_rect = split_rect(&mut rect, line.indent_level * INDENT_AMOUNT);

            if let Some(name) = line.name {
                let name_rect = split_rect(&mut rect, name.len() as u16 + 2);
                Line::from(vec![Span::raw(name), Span::raw(": ")])
                    .bold()
                    .fg(if selected {
                        PRIMARY_COLOR
                    } else {
                        Color::Reset
                    })
                    .render(name_rect, buf);
            }

            match &line.kind {
                InspectorLineKind::ObjectStart => render_char(rect, buf, '{', selected),
                InspectorLineKind::ObjectEnd => render_char(rect, buf, '}', selected),

                InspectorLineKind::ArrayStart => render_char(rect, buf, '[', selected),
                InspectorLineKind::ArrayEnd => render_char(rect, buf, ']', selected),

                InspectorLineKind::Item { value } => {
                    let span = match value {
                        PrimitiveValue::Null => Span::raw("None"),
                        PrimitiveValue::Bool(b) => Span::raw(b.to_string()),
                        PrimitiveValue::Number(n) => Span::raw(n.to_string()),
                        PrimitiveValue::String(s) => Span::raw(*s),
                    };
                    if selected {
                        span.fg(PRIMARY_COLOR).bold().render(rect, buf);
                    } else {
                        span.render(rect, buf);
                    };
                }
            }
        }
    }
}

impl InspectorState {
    pub fn select_previous(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn select_next(&mut self) {
        self.selected = (self.selected + 1).min(self.value_types.len() - 1);
    }

    pub fn selected_path(&self) -> &str {
        &self.paths[self.selected]
    }

    pub fn selected_value_type(&self) -> ValueType {
        self.value_types[self.selected]
    }

    fn update_paths(&mut self, flat_map: &[InspectorLine]) {
        self.paths = flat_map
            .iter()
            .filter(|line| line.selectable())
            .map(|line| line.path.clone())
            .collect()
    }

    fn update_value_types(&mut self, flat_map: &[InspectorLine]) {
        self.value_types = flat_map
            .iter()
            .filter_map(InspectorLine::value_type)
            .collect();
    }

    fn update_scroll(&mut self, flat_map: &[InspectorLine], height: u16) {
        let selected_line_y = flat_map
            .iter()
            .enumerate()
            .filter(|(_, l)| l.selectable())
            .nth(self.selected)
            .map(|(y, _)| y)
            .unwrap_or_default();
        if selected_line_y < self.scroll + 6 {
            self.scroll = self.scroll.saturating_sub(1);
        }
        if selected_line_y > self.scroll + height.saturating_sub(6) as usize {
            self.scroll += 1;
        }
        self.scroll = self
            .scroll
            .min(flat_map.len().saturating_sub(height as usize));
    }

    fn update_selected(&mut self, flat_map: &[InspectorLine]) {
        self.selected = self.selected.min(flat_map.len().saturating_sub(1));
    }
}

#[derive(Debug)]
struct InspectorLine<'a> {
    name: Option<&'a str>,
    path: String,
    indent_level: u16,
    kind: InspectorLineKind<'a>,
}

#[derive(Debug)]
enum InspectorLineKind<'a> {
    ObjectStart,
    ArrayStart,
    Item { value: PrimitiveValue<'a> },
    ArrayEnd,
    ObjectEnd,
}

/// A copy of [`Value`] with just the types that are primitive in Rust.
#[derive(Debug)]
enum PrimitiveValue<'a> {
    Null,
    Bool(bool),
    Number(Number),
    String(&'a str),
}

fn flatten_value(value: &Value) -> Vec<InspectorLine> {
    let mut flat_map = Vec::new();
    flatten_value_inner(None, value, &mut flat_map, String::new(), 0);
    flat_map
}

fn flatten_value_inner<'a>(
    name: Option<&'a str>,
    value: &'a Value,
    out: &mut Vec<InspectorLine<'a>>,
    base_path: String,
    indent_level: u16,
) {
    match value {
        Value::Null => out.push(InspectorLine {
            name,
            path: base_path,
            indent_level,
            kind: InspectorLineKind::Item {
                value: PrimitiveValue::Null,
            },
        }),
        Value::Bool(b) => out.push(InspectorLine {
            name,
            path: base_path,
            indent_level,
            kind: InspectorLineKind::Item {
                value: PrimitiveValue::Bool(*b),
            },
        }),
        Value::Number(n) => out.push(InspectorLine {
            name,
            path: base_path,
            indent_level,
            kind: InspectorLineKind::Item {
                value: PrimitiveValue::Number(n.to_owned()),
            },
        }),
        Value::String(s) => out.push(InspectorLine {
            name,
            path: base_path,
            indent_level,
            kind: InspectorLineKind::Item {
                value: PrimitiveValue::String(s),
            },
        }),

        Value::Array(array) => {
            out.push(InspectorLine {
                name,
                path: base_path.to_owned(),
                indent_level,
                kind: InspectorLineKind::ArrayStart,
            });
            for (n, value) in array.iter().enumerate() {
                flatten_value_inner(
                    None,
                    value,
                    out,
                    format!("{base_path}[{n}]"),
                    indent_level + 1,
                );
            }
            out.push(InspectorLine {
                name: None,
                path: base_path,
                indent_level,
                kind: InspectorLineKind::ArrayEnd,
            });
        }

        Value::Object(map) => {
            out.push(InspectorLine {
                name,
                path: base_path.to_owned(),
                indent_level,
                kind: InspectorLineKind::ObjectStart,
            });
            for (name, value) in map {
                flatten_value_inner(
                    Some(name),
                    value,
                    out,
                    format!("{base_path}.{name}"),
                    indent_level + 1,
                );
            }
            out.push(InspectorLine {
                name: None,
                path: base_path,
                indent_level,
                kind: InspectorLineKind::ObjectEnd,
            });
        }
    }
}

impl InspectorLine<'_> {
    /// The [`ValueType`] of this line to determine which keybinds to show.
    fn value_type(&self) -> Option<ValueType> {
        match &self.kind {
            InspectorLineKind::Item { value } => Some(ValueType::from(value)),
            InspectorLineKind::ArrayStart => Some(ValueType::Array),
            InspectorLineKind::ObjectStart => Some(ValueType::Object),
            _ => None,
        }
    }

    /// If this line should be able to be selected.
    fn selectable(&self) -> bool {
        self.value_type().is_some()
    }
}

/// Take the given `width` off the front of the given `rect` and return a new rect containing that
/// space.
fn split_rect(rect: &mut Rect, width: u16) -> Rect {
    let new_rect = Rect { width, ..*rect };
    rect.width -= width;
    rect.x += width;
    new_rect
}

fn render_char(rect: Rect, buf: &mut Buffer, ch: char, selected: bool) {
    buf[rect.as_position()].set_char(ch);
    if selected {
        buf[rect.as_position()].set_style(Style::default().fg(PRIMARY_COLOR).bold());
    }
}

impl From<&PrimitiveValue<'_>> for ValueType {
    fn from(value: &PrimitiveValue) -> Self {
        match value {
            PrimitiveValue::Null => Self::Null,
            PrimitiveValue::Bool(_) => Self::Bool,
            PrimitiveValue::Number(_) => Self::Number,
            PrimitiveValue::String(_) => Self::String,
        }
    }
}
