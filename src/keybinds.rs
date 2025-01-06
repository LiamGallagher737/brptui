use ratatui::{
    prelude::{Buffer, Rect},
    style::{Style, Stylize},
    text::{Line, Span},
    widgets::Widget,
};

#[derive(Debug, Default)]
pub struct Keybinds {
    keybinds: Vec<(String, String)>,
}

impl Keybinds {
    pub fn push(&mut self, keys: impl Into<String>, description: impl Into<String>) -> &mut Self {
        self.keybinds.push((keys.into(), description.into()));
        self
    }
}

impl Widget for Keybinds {
    fn render(mut self, area: Rect, buf: &mut Buffer) {
        let keybinds_len = self.keybinds.len();
        let text = Line::from(
            self.keybinds
                .drain(..)
                .enumerate()
                .flat_map(|(n, (key, description))| {
                    let dim = Style::default().dim();
                    [
                        Span::styled(key, dim.bold()),
                        Span::raw(" "),
                        Span::styled(description, dim),
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
