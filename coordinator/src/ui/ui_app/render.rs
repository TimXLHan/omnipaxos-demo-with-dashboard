
use crate::messages::coordinator::Round;
use ratatui::backend::Backend;
use ratatui::symbols::Marker;
use ratatui::text::{Span, Line};
use ratatui::widgets::canvas::{Canvas, Rectangle};
use ratatui::widgets::{
    block::Block, BarChart, BorderType, Borders, Gauge, Paragraph,
};
use ratatui::Frame;
use ratatui::{
    prelude::*,
    widgets::{block::Title, *},
};
use std::collections::{HashMap};
use std::f64::consts::PI;

use tui_textarea::TextArea;

use crate::ui::ui_app::UIApp;
use crate::utils::{
    UI_BARCHART_GAP, UI_BARCHART_WIDTH, UI_INPUT_AREA_TITLE, UI_LEADER_RECT_COLOR,
    UI_OUTPUT_AREA_TITLE, UI_PROGRESS_BAR_TITLE, UI_THROUGHPUT_TITLE, UI_TITLE,
};

/// render ui components
pub fn render<B>(rect: &mut Frame<B>, app: &UIApp)
where
    B: Backend,
{
    let size = rect.size();
    let window_width: usize = size.width.into();

    // Vertical layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                // Title
                Constraint::Length(3),
                // Bar Chart
                Constraint::Length(10),
                // Progress Bar
                Constraint::Length(3),
                // Output & Connection Status
                Constraint::Min(10),
                // Input
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(size);

    // Title
    let title = draw_title(app);
    rect.render_widget(title, chunks[0]);

    // Chart
    // let tooltip =     Canvas::default()
    //     .block(Block::default().borders(Borders::ALL).title("World"))
    //     .paint(|ctx| {
    //         ctx.print(
    //             0.0,
    //             0.0,
    //             Span::styled("You are here", Style::default().fg(Color::Yellow)),
    //         );
    //     })
    //     .x_bounds([0.0, 100.0])
    //     .y_bounds([0.0, 100.0]);
    // rect.render_widget(tooltip, chunks[1]);

    let chart_data: &Vec<(&str, u64)> = &app
        .throughput_data
        .iter()
        .take(window_width / (UI_BARCHART_WIDTH + UI_BARCHART_GAP) as usize)
        .map(|(s, num)| (s.as_str(), *num))
        .collect::<Vec<(&str, u64)>>();
    let chart = draw_chart(app, chart_data);
    rect.render_widget(chart, chunks[1]);

    // Progress Bar
    let progress_bar = draw_progress_bar(app);
    rect.render_widget(progress_bar, chunks[2]);

    // Output & Status
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(80), Constraint::Min(20)].as_ref())
        .split(chunks[3]);

    // Output
    let output = draw_output(
        app,
        body_chunks[0].height as i64 - 2,
        body_chunks[0].width as i64,
    );
    rect.render_widget(output, body_chunks[0]);

    let canvas_node = Canvas::default()
        .block(Block::default().title("Canvas").borders(Borders::ALL))
        .marker(Marker::Braille)
        .x_bounds([-90.0, 90.0])
        .y_bounds([-60.0, 60.0])
        .paint(|ctx| {
            let canvas_components = make_canvas(app);
            for node_rect in canvas_components.nodes.values() {
                if node_rect.color == UI_LEADER_RECT_COLOR {
                    ctx.draw(node_rect);
                }
            }

            for label in canvas_components.labels.values() {
                ctx.print(label.x, label.y, label.span.clone());
            }
        });
    let canvas_line_lable = Canvas::default()
        .block(Block::default().title("Canvas").borders(Borders::ALL))
        .marker(Marker::Braille)
        .x_bounds([-90.0, 90.0])
        .y_bounds([-60.0, 60.0])
        .paint(|ctx| {
            let canvas_components = make_canvas(app);

            for line in canvas_components.connections.values() {
                ctx.draw(line);
            }
            for label in canvas_components.labels.values() {
                ctx.print(label.x, label.y, label.span.clone());
            }
        });
    rect.render_widget(canvas_line_lable, body_chunks[1]);
    rect.render_widget(canvas_node, body_chunks[1]);

    // Input
    let textarea = app.input_area.clone();
    let input = draw_input(textarea);
    rect.render_widget(input.widget(), chunks[4]);
}

struct CanvasComponents {
    nodes: HashMap<u64, Rectangle>,
    connections: HashMap<(u64, u64), canvas::Line>,
    labels: HashMap<u64, Label<'static>>,
}

struct Label<'a> {
    x: f64,
    y: f64,
    span: Span<'a>,
}

fn make_canvas(app: &UIApp) -> CanvasComponents {
    if app.nodes.is_empty() {
        return CanvasComponents {
            nodes: HashMap::new(),
            connections: HashMap::new(),
            labels: HashMap::new(),
        };
    }
    let network_status = &app.network_state;
    let num_of_nodes = network_status.alive_nodes.len();
    let node_width = 15.0;
    let radius = 50.0; // Radius of the circle
    let center_x = -node_width / 2.0; // X-coordinate of the circle's center
    let center_y = -node_width / 2.0; // Y-coordinate of the circle's center

    let angle_step = 2.0 * PI / (num_of_nodes as f64); // Angle increment between each rectangle
    let mut nodes_with_rects = HashMap::new();

    // Rectangles, but only shows the leader
    for i in 0..num_of_nodes {
        let angle = i as f64 * angle_step;
        let x = center_x + radius * angle.cos();
        let y = center_y + radius * angle.sin();
        let node_id = network_status.alive_nodes[i];
        let color = match network_status.max_round {
            Some(Round { leader: l, .. }) if node_id == l => UI_LEADER_RECT_COLOR,
            _ => Color::Reset,
        };
        // let rect = Rectangle::new(Point::new(x, y), 1.0, 1.0); // Adjust the width and height as desired
        let rect = Rectangle {
            x,
            y,
            width: node_width * 2.0,
            height: node_width,
            color,
        };
        nodes_with_rects.insert(node_id, rect);
    }

    // Connections
    let mut lines = HashMap::new();
    for i in 0..network_status.alive_nodes.len() {
        for j in i..network_status.alive_nodes.len() {
            let node1 = network_status.alive_nodes.get(i).unwrap();
            let node2 = network_status.alive_nodes.get(j).unwrap();
            let current_rect = nodes_with_rects.get(node1).unwrap();
            let next_rect = nodes_with_rects.get(node2).unwrap();

            if network_status.partitions.get(&(*node1, *node2)).is_none() {
                let line = canvas::Line {
                    x1: current_rect.x + current_rect.width / 2.0,
                    y1: current_rect.y + current_rect.height / 2.0,
                    x2: next_rect.x + next_rect.width / 2.0,
                    y2: next_rect.y + next_rect.height / 2.0,
                    color: Color::White,
                };
                lines.insert((i as u64, j as u64), line);
            }
        }
    }

    // Labels
    let mut labels = HashMap::new();
    for (node_id, rect) in &nodes_with_rects {
        let node = app.nodes.iter().find(|node| node.pid == *node_id).unwrap();
        let label = Label {
            x: rect.x + rect.width / 4.0,
            y: rect.y + rect.width / 6.0,
            span: Span::styled(
                " Node".to_string() + &*node_id.to_string() + " ",
                Style::default().fg(Color::White).bold().bg(node.color),
            ),
        };
        labels.insert(*node_id, label);
    }

    CanvasComponents {
        nodes: nodes_with_rects,
        connections: lines,
        labels,
    }
}

fn draw_title<'a>(_app: &UIApp) -> Paragraph<'a> {
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

fn draw_chart<'a>(app: &UIApp, data: &'a Vec<(&'a str, u64)>) -> BarChart<'a> {
    let leader = app.leader.clone().unwrap_or_default();
    let title = Title::from(Line::from(vec![
        Span::styled(
            format!("{}: {:3} req/s", UI_THROUGHPUT_TITLE, app.dps),
            Style::default()
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            " (# round number)",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    BarChart::default()
        .block(Block::default().title(title).borders(Borders::ALL))
        .data(data)
        .bar_width(UI_BARCHART_WIDTH)
        .bar_gap(UI_BARCHART_GAP)
        .value_style(Style::default().fg(leader.color).bg(leader.color))
        .label_style(Style::default().fg(Color::Yellow))
        .bar_style(Style::default().fg(leader.color))
}
fn draw_progress_bar<'a>(app: &UIApp) -> Gauge<'a> {
    let (progress, total) = (app.progress.finished, app.progress.total);
    let label = format!("{}/{}", progress, total);
    let is_ongoing = app.progress.is_ongoing;
    let bar_color = if is_ongoing {
        Color::LightGreen
    } else {
        Color::Gray
    };
    Gauge::default()
        .block(
            Block::default()
                .title(UI_PROGRESS_BAR_TITLE)
                .borders(Borders::ALL),
        )
        .gauge_style(
            Style::default()
                .fg(bar_color)
                .bg(Color::Black)
                .add_modifier(Modifier::ITALIC),
        )
        .percent(((progress * 100 / total) as u16).min(100))
        .label(label)
}

fn draw_input(mut textarea: TextArea) -> TextArea {
    textarea.set_style(Style::default().fg(Color::LightGreen));
    textarea.set_block(
        Block::default()
            .borders(Borders::ALL)
            .title(UI_INPUT_AREA_TITLE),
    );

    textarea
}

fn draw_output<'a>(app: &UIApp, block_height: i64, _block_width: i64) -> Paragraph<'a> {
    // let logs = reformat_output(app.get_logs(), block_width as u64);
    let logs = app.get_logs();
    let log_len = logs.len();
    Paragraph::new(
        logs.into_iter()
            .map(|s| Line::from(Span::raw(s)))
            .collect::<Vec<_>>(),
    )
    .style(Style::default().fg(Color::LightCyan))
    .alignment(Alignment::Left)
    .scroll((
        (log_len as i64 - block_height + app.scroll).max(0) as u16,
        0,
    ))
    .block(
        Block::default()
            // .title("Body")
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .border_type(BorderType::Plain)
            .title(UI_OUTPUT_AREA_TITLE),
    )
}

#[allow(dead_code)]
// This is a temporary solution to the problem of long lines in the output area.
fn reformat_output(logs: Vec<String>, block_width: u64) -> Vec<String> {
    let mut result = Vec::new();
    for log in logs {
        let mut lines = log.split('\n').collect::<Vec<_>>();
        for line in lines.iter_mut() {
            let mut chunks = split_string_by_length(line, block_width as usize);
            result.append(&mut chunks);
        }
    }
    result
}

fn split_string_by_length(input: &str, length: usize) -> Vec<String> {
    let mut result = Vec::new();
    let mut remaining = input;

    while !remaining.is_empty() {
        let end_index = std::cmp::min(length, remaining.len());
        let chunk = remaining[..end_index].to_owned();
        result.push(chunk.clone());
        remaining = &remaining[end_index..];
    }

    result
}
