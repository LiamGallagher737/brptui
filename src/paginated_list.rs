use ratatui::{
    prelude::{BlockExt, Buffer, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::{Block, StatefulWidget, Widget},
};

use crate::PRIMARY_COLOR;

#[derive(Debug)]
pub struct PaginatedList<'a> {
    block: Option<Block<'a>>,
    items: Vec<Line<'a>>,
    focused: bool,
}

impl<'a> PaginatedList<'a> {
    pub fn new<T: IntoIterator<Item = Line<'a>>>(items: T, focused: bool) -> Self {
        Self {
            items: items.into_iter().map(Into::into).collect(),
            block: None,
            focused,
        }
    }

    pub fn block(mut self, block: Block<'a>) -> Self {
        self.block = Some(block);
        self
    }
}

#[derive(Debug, Default)]
pub struct PaginatedListState {
    selected: usize,
    cursor_move: Option<CursorMove>,
}

#[derive(Debug)]
enum CursorMove {
    Previous,
    Next,
    PreviousPage,
    NextPage,
}

impl PaginatedListState {
    pub fn selected(&self) -> usize {
        self.selected
    }

    pub fn select_previous(&mut self) {
        assert!(self.cursor_move.is_none(), "cursor_move is set");
        self.cursor_move = Some(CursorMove::Previous);
    }

    pub fn select_next(&mut self) {
        assert!(self.cursor_move.is_none(), "cursor_move is set");
        self.cursor_move = Some(CursorMove::Next);
    }

    pub fn select_previous_page(&mut self) {
        assert!(self.cursor_move.is_none(), "cursor_move is set");
        self.cursor_move = Some(CursorMove::PreviousPage);
    }

    pub fn select_next_page(&mut self) {
        assert!(self.cursor_move.is_none(), "cursor_move is set");
        self.cursor_move = Some(CursorMove::NextPage);
    }

    fn apply_cursor_move(&mut self, per_page: usize, items: usize) {
        let total_pages = items.div_ceil(per_page);
        match self.cursor_move {
            Some(CursorMove::Previous) if self.selected == 0 => self.selected = items - 1,
            Some(CursorMove::Previous) => self.selected -= 1,

            Some(CursorMove::Next) if self.selected == items - 1 => self.selected = 0,
            Some(CursorMove::Next) => self.selected += 1,

            Some(CursorMove::PreviousPage) if self.selected < per_page => {
                self.selected += per_page * (total_pages - 1)
            }
            Some(CursorMove::PreviousPage) => self.selected -= per_page,

            Some(CursorMove::NextPage) if self.selected >= per_page * (total_pages - 1) => {
                self.selected -= per_page * (total_pages - 1)
            }
            Some(CursorMove::NextPage) => self.selected += per_page,

            None => {}
        }
        self.selected = self.selected.min(items - 1);
        self.cursor_move = None;
    }
}

impl StatefulWidget for PaginatedList<'_> {
    type State = PaginatedListState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        if let Some(block) = &self.block {
            block.render(area, buf);
        }
        let area = self.block.inner_if_some(area);

        let per_page = area.height as usize - 2;
        let total_pages = self.items.len().div_ceil(per_page);

        state.apply_cursor_move(per_page, self.items.len());

        let page = state.selected / per_page;
        let page_items =
            &self.items[page * per_page..(page * per_page + per_page).min(self.items.len())];
        let page_selected = state.selected - page * per_page;

        // Render items
        for (n, line) in page_items.iter().enumerate() {
            let mut item_area = Rect {
                height: 1,
                width: area.width,
                x: area.x,
                y: area.y + n as u16,
            };
            if n != page_selected {
                line.render(item_area, buf);
            } else {
                let style = if self.focused {
                    Style::default().fg(PRIMARY_COLOR)
                } else {
                    Style::default()
                };
                buf[item_area.as_position()]
                    .set_char('>')
                    .set_style(style.bold());
                item_area.x += 2;
                item_area.width -= 2;
                line.clone().style(style).render(item_area, buf);
            }
        }

        // Render pagination
        if total_pages > 1 {
            let line = Line::from(
                (0..total_pages)
                    .map(|n| {
                        if n != page {
                            Span::raw("• ").bold().dim()
                        } else {
                            Span::raw("• ").bold()
                        }
                    })
                    .collect::<Vec<Span>>(),
            );
            buf.set_line(area.x, area.y + area.height - 1, &line, area.width);
        }
    }
}
