use std::ops::Deref;
use ratatui::backend::Backend;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::{Span, Spans};
use ratatui::widgets::{Block, Borders, BorderType, Paragraph, Wrap};
use tui_textarea::{Input, TextArea};

use crate::ui::ui_app::UIApp;
use crate::utils::{UI_INPUT_AREA_TITLE, UI_OUTPUT_AREA_TITLE, UI_TITLE};

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

    // Output & Status
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length(32)].as_ref())
        .split(chunks[2]);

    // Output
    let output = draw_output(app, body_chunks[0].height as i64 - 2);
    rect.render_widget(output, body_chunks[0]);

    // Input
    let mut textarea = app.input_area.clone();
    let input = draw_input(textarea);
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


fn draw_input(mut textarea: TextArea) -> TextArea {
    textarea.set_style(Style::default().fg(Color::LightGreen));
    textarea.set_block(Block::default().borders(Borders::ALL).title(UI_INPUT_AREA_TITLE));

    textarea
}

fn draw_output<'a>(app: &UIApp, block_height: i64) -> Paragraph<'a> {
    let logs = app.get_logs();
    let log_len = logs.len();
    Paragraph::new(logs.into_iter().map(|s| Spans::from(Span::raw(s))).collect::<Vec<_>>())
        .style(Style::default().fg(Color::LightCyan))
        .alignment(Alignment::Left)
        .wrap(Wrap { trim: true })
        .scroll(((log_len as i64 - block_height + app.scroll).max(0) as u16, 0))
        .block(
            Block::default()
                // .title("Body")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                .border_type(BorderType::Plain)
                .title(UI_OUTPUT_AREA_TITLE),
        )
}