use ratatui::{
    prelude::{BlockExt, Buffer, Rect},
    widgets::{Block, Paragraph, StatefulWidget, Widget},
};
use serde_json::Value;

pub struct Inspector<'a> {
    value: &'a Value,
    block: Option<Block<'a>>,
}

impl<'a> Inspector<'a> {
    pub fn new(value: &'a Value) -> Self {
        Self { value, block: None }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }
}

#[derive(Debug, Default)]
pub struct InspectorState {}

impl StatefulWidget for Inspector<'_> {
    type State = InspectorState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if let Some(block) = &self.block {
            block.render(area, buf);
        }
        let area = self.block.inner_if_some(area);

        Paragraph::new(format!("{:#?}", self.value)).render(area, buf);
    }
}
