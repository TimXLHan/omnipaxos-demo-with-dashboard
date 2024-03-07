use crate::coordinator::NetworkState;
use crate::messages::coordinator::Round;
use ratatui::backend::Backend;
use ratatui::layout::{Alignment, Constraint, Direction, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::symbols::Marker;
use ratatui::text::{Span, Spans};
use ratatui::widgets::canvas::{Canvas, Context, Line, Rectangle};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph, Sparkline, Wrap, BarChart, Gauge};
use ratatui::Frame;
use std::collections::{HashMap, VecDeque};
use std::f64::consts::PI;
use tui_textarea::TextArea;

use crate::ui::ui_app::UIApp;
use crate::utils::{UI_BARCHART_GAP, UI_BARCHART_WIDTH, UI_INPUT_AREA_TITLE, UI_OUTPUT_AREA_TITLE, UI_PROGRESS_BAR_TITLE, UI_THROUGHPUT_TITLE, UI_TITLE};

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
    let title = draw_title();
    rect.render_widget(title, chunks[0]);

    // Chart
    let chart_data: &Vec<(&str, u64)> = &app
        .throughput_data
        .iter()
        .take(window_width / (UI_BARCHART_WIDTH + UI_BARCHART_GAP) as usize)
        .map(|(s, num)| (s.as_str(), *num))
        .collect::<Vec<(&str, u64)>>();
    let chart = draw_chart(chart_data);
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
    let output = draw_output(app, body_chunks[0].height as i64 - 2);
    rect.render_widget(output, body_chunks[0]);

    let canvas_node = Canvas::default()
        .block(Block::default().title("Canvas").borders(Borders::ALL))
        .marker(Marker::Braille)
        .x_bounds([-90.0, 90.0])
        .y_bounds([-60.0, 60.0])
        .paint(|ctx| {
            let canvas_components = make_canvas(&app.network_state);
            for node in canvas_components.nodes.values() {
                ctx.draw(node);
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
            let canvas_components = make_canvas(&app.network_state);

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
    let mut textarea = app.input_area.clone();
    let input = draw_input(textarea);
    rect.render_widget(input.widget(), chunks[4]);
}

struct CanvasComponents {
    nodes: HashMap<u64, Rectangle>,
    connections: HashMap<(u64, u64), Line>,
    labels: HashMap<u64, Label<'static>>,
}

struct Label<'a> {
    x: f64,
    y: f64,
    span: Span<'a>,
}

fn make_canvas(network_status: &NetworkState) -> CanvasComponents {
    let num_of_nodes = network_status.alive_nodes.len();
    let node_width = 15.0;
    let radius = 50.0; // Radius of the circle
    let center_x = -node_width / 2.0; // X-coordinate of the circle's center
    let center_y = -node_width / 2.0; // Y-coordinate of the circle's center

    let angle_step = 2.0 * PI / (num_of_nodes as f64); // Angle increment between each rectangle
    let mut nodes_with_rects = HashMap::new();

    // Nodes
    for i in 0..num_of_nodes {
        let angle = i as f64 * angle_step;
        let x = center_x + radius * angle.cos();
        let y = center_y + radius * angle.sin();
        let node_id = network_status.alive_nodes[i];
        let color = match network_status.max_round {
            Some(Round { leader: l, .. }) if node_id == l => Color::Green,
            _ => Color::White,
        };
        // let rect = Rectangle::new(Point::new(x, y), 1.0, 1.0); // Adjust the width and height as desired
        let rect = Rectangle {
            x,
            y,
            width: node_width,
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

            if let None = network_status.partitions.get(&(*node1, *node2)) {
                let line = Line {
                    x1: current_rect.x + current_rect.width / 2.0,
                    y1: current_rect.y + current_rect.height / 2.0,
                    x2: next_rect.x + next_rect.width / 2.0,
                    y2: next_rect.y + next_rect.height / 2.0,
                    color: Color::LightBlue,
                };
                lines.insert((i as u64, j as u64), line);
            }
        }
    }

    // Labels
    let mut labels = HashMap::new();
    for (node_id, rect) in &nodes_with_rects {
        let happy = network_status.happiness.get(node_id);
            let happy_label = match happy {
            Some(true) => " :D",
            Some(false) => " :(",
            None => "",
        };
        let label = Label {
            x: rect.x + rect.width / 4.0,
            y: rect.y + rect.width / 3.0,
            span: Span::styled(
                String::from("Node".to_string() + &*node_id.to_string() + happy_label),
                Style::default().fg(Color::White),
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

fn draw_chart<'a>(data: &'a Vec<(&'a str, u64)>) -> BarChart<'a> {
    BarChart::default()
        .block(
            Block::default()
                .title(UI_THROUGHPUT_TITLE)
                .borders(Borders::ALL),
        )
        .data(data)
        .bar_width(UI_BARCHART_WIDTH)
        .bar_gap(UI_BARCHART_GAP)
        .value_style(Style::default().fg(Color::Black).bg(Color::LightGreen))
        .style(Style::default().fg(Color::LightGreen))
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
        .block(Block::default().title(UI_PROGRESS_BAR_TITLE).borders(Borders::ALL))
        .gauge_style(
            Style::default()
                .fg(bar_color)
                .bg(Color::Black)
                .add_modifier(Modifier::ITALIC),
        )
        .percent(((progress as u64  * 100 / total as u64) as u16).min(100))
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

fn draw_output<'a>(app: &UIApp, block_height: i64) -> Paragraph<'a> {
    let logs = app.get_logs();
    let log_len = logs.len();
    Paragraph::new(
        logs.into_iter()
            .map(|s| Spans::from(Span::raw(s)))
            .collect::<Vec<_>>(),
    )
    .style(Style::default().fg(Color::LightCyan))
    .alignment(Alignment::Left)
    .wrap(Wrap { trim: true })
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
