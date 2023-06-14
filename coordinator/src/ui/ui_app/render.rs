use tui::backend::Backend;
use tui::Frame;
use tui::layout::{Alignment, Constraint, Direction, Layout};
use tui::style::{Color, Style};
use tui::widgets::{Block, Borders, BorderType, Paragraph};
use tui_textarea::{Input, Key, TextArea};

use crate::ui::ui_app::UIApp;
use crate::utils::{UI_INPUT_AREA_TITLE, UI_TITLE};

/// render ui components
pub fn render<B>(rect: &mut Frame<B>, app: &UIApp)
    where
        B: Backend,
{
    let size = rect.size();

    // Vertical layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                // Title
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(12),
                // Input
                Constraint::Length(3),
            ]
                .as_ref(),
        )
        .split(size);

    // Title
    let title = draw_title();
    rect.render_widget(title, chunks[0]);

    // Input
    let input = draw_input();
    rect.render_widget(input.widget(), chunks[3]);
}

fn draw_title<'a>() -> Paragraph<'a> {
    Paragraph::new(UI_TITLE)
        .style(Style::default().fg(Color::LightCyan))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                .border_type(BorderType::Plain),
        )
}


fn draw_input<'a>() -> TextArea<'a> {
    let mut textarea = TextArea::default();
    textarea.set_cursor_line_style(Style::default());
    textarea.set_style(Style::default().fg(Color::LightGreen));
    textarea.set_block(Block::default().borders(Borders::ALL).title(UI_INPUT_AREA_TITLE));
    // temp
    let input = Input { key: Key::Char('a'), ctrl: false, alt: false };
    textarea.input(input);
    textarea
}