use std::ops::Deref;
use ratatui::backend::Backend;
use ratatui::Frame;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Style};
use ratatui::text::{Span, Spans};
use ratatui::widgets::{Block, Borders, BorderType, Paragraph, Sparkline, Wrap};
use ratatui::widgets::canvas::{Canvas, Context, Line, Rectangle};
use tui_textarea::{Input, TextArea};

use crate::ui::ui_app::UIApp;
use crate::utils::{UI_INPUT_AREA_TITLE, UI_OUTPUT_AREA_TITLE, UI_Throughput_TITLE, UI_TITLE};

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
                // Sparkline
                Constraint::Length(10),
                // Output & Connection Status
                Constraint::Min(10),
                // Input
                Constraint::Length(3),
            ]
                .as_ref(),
        )
        .split(size);

    // Title
    let title = draw_title();
    rect.render_widget(title, chunks[0]);

    // Sparkline
    let sparkline = draw_sparkline();
    rect.render_widget(sparkline, chunks[1]);

    // Output & Status
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(80), Constraint::Min(20)].as_ref())
        .split(chunks[2]);

    // Output
    let output = draw_output(app, body_chunks[0].height as i64 - 2);
    rect.render_widget(output, body_chunks[0]);

    // Connection Status
    let data = Rectangle {
        x: 40.0,
        y: 20.0,
        width: 5.0,
        height: 5.0,
        color: Color::Red,
    };
    let connection_status = draw_connection_status(&data);
    let canvas = Canvas::default()
        .block(Block::default().title("Canvas").borders(Borders::ALL))
        .x_bounds([-90.0, 90.0])
        .y_bounds([-60.0, 60.0])
        .paint(|ctx| {
            ctx.draw(&data);
        });
    rect.render_widget(canvas, body_chunks[1]);

    // Input
    let mut textarea = app.input_area.clone();
    let input = draw_input(textarea);
    rect.render_widget(input.widget(), chunks[3]);
}

fn draw_connection_status(data: &Rectangle) -> Canvas<'static, fn(&mut Context)> {
    Canvas::default()
        .block(Block::default().title("Canvas").borders(Borders::ALL))
        .x_bounds([-180.0, 180.0])
        .y_bounds([-90.0, 90.0])
        .paint(|ctx| {
            // println!("{:?}",&data);
            ctx.draw(&Line {
                x1: 0.0,
                y1: 10.0,
                x2: 10.0,
                y2: 10.0,
                color: Color::White,
            });
            // ctx.draw(&data);
            ctx.draw(&Rectangle {
                x: 10.0,
                y: 20.0,
                width: 10.0,
                height: 10.0,
                color: Color::Red,
            });
        })
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

fn draw_sparkline<'a>() -> Sparkline<'a> {
    Sparkline::default()
        .block(
            Block::default()
                .title(UI_Throughput_TITLE)
                .borders(Borders::ALL),
        )
        .data(&[1, 2, 5, 3, 4, 1, 2, 5, 3, 4, 1, 2, 5, 3, 4, 1, 2, 5, 3, 4, 1, 2, 5, 3, 4, 1, 2, 5, 3, 4, 5])
        .style(Style::default().fg(Color::Green))
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