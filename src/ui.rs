use crate::actions::Actions;
use crate::app::App;
use crate::app::AppState;

use tui::backend::Backend;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::symbols::line;
use tui::text::{Span, Spans};
use tui::widgets::{
    Block, BorderType, Borders, Cell, LineGauge, List, ListItem, Paragraph, Row, Table,
};
use tui::Frame;

use std::time::Duration;

use tui_logger::TuiLoggerWidget;

use yandex_rust_music::Track;

pub fn draw<B>(rect: &mut Frame<B>, app: &mut App)
where
    B: Backend,
{
    let size = rect.size();
    check_size(&size);

    // Vertical layout
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(
            [
                Constraint::Length(3),
                Constraint::Min(10),
                Constraint::Length(3),
                Constraint::Length(12),
            ]
            .as_ref(),
        )
        .split(size);

    // Title
    let title = draw_title();
    rect.render_widget(title, chunks[0]);

    // Body (player and state) and Help
    let body_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(20), Constraint::Length(32)].as_ref())
        .split(chunks[1]);

    let player_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(3), Constraint::Length(5)].as_ref())
        .split(body_chunks[0]);

    // let playlist = draw_playlist(app.current_playlist());
    // rect.render_widget(playlist, player_chunks[0]);
    // FIXME: can we avoid clone?
    // let playlist = draw_tracks(&displayed_tracks);
    let tracks: Vec<ListItem> = app
        .displayed_tracks
        .tracks
        .iter()
        .map(|i| ListItem::new(vec![Spans::from(i.title())]))
        .collect();
    let list = List::new(tracks)
        .block(Block::default().borders(Borders::ALL).title("List"))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");
    rect.render_stateful_widget(list, player_chunks[0], &mut app.displayed_tracks.state);

    let state = draw_body(app.is_loading(), app.state());
    rect.render_widget(state, player_chunks[1]);

    let help = draw_help(app.actions());
    rect.render_widget(help, body_chunks[1]);

    // Duration
    if let Some(duration) = app.state().duration() {
        let total_duration = app.state().total_duration().unwrap();
        let duration_block = draw_duration(duration, total_duration);
        rect.render_widget(duration_block, chunks[2]);
    }

    // Logs
    let logs = draw_logs();
    rect.render_widget(logs, chunks[3]);
}

fn check_size(rect: &Rect) {
    if rect.width < 52 {
        panic!("Require width >= 52, (got {})", rect.width);
    }

    if rect.height < 28 {
        panic!("Require height >= 28, (got {})", rect.height);
    }
}
fn draw_title<'a>() -> Paragraph<'a> {
    Paragraph::new("Yandex Music CLI")
        .style(Style::default().fg(Color::LightCyan))
        .alignment(Alignment::Center)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::White))
                .border_type(BorderType::Plain),
        )
}

fn draw_body<'a>(loading: bool, state: &AppState) -> Paragraph<'a> {
    let initialized_text = if state.is_initialized() {
        "Initialized"
    } else {
        "Not Initialized"
    };
    let loading_text = if loading { "Loading..." } else { "" };

    Paragraph::new(vec![
        Spans::from(Span::raw(initialized_text)),
        Spans::from(Span::raw(loading_text)),
    ])
    .style(Style::default().fg(Color::LightCyan))
    .alignment(Alignment::Left)
    .block(
        Block::default()
            .borders(Borders::ALL)
            .style(Style::default().fg(Color::White))
            .border_type(BorderType::Plain),
    )
}

fn draw_playlist(playlist: &Vec<Track>) -> List {
    let songs: Vec<ListItem> = playlist
        .iter()
        .map(|i| ListItem::new(vec![Spans::from(i.title())]))
        .collect();
    let songs = List::new(songs)
        .block(Block::default().borders(Borders::ALL).title("List"))
        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
        .highlight_symbol("> ");

    songs
}

// fn draw_tracks(playlist: &DisplayedTracks) -> List {
//     let tracks: Vec<ListItem> = playlist
//         .tracks
//         .iter()
//         .map(|i| ListItem::new(vec![Spans::from(i.title())]))
//         .collect();
//     let list = List::new(tracks)
//         .block(Block::default().borders(Borders::ALL).title("List"))
//         .highlight_style(Style::default().add_modifier(Modifier::BOLD))
//         .highlight_symbol("> ");
//     list
// }

fn draw_help(actions: &Actions) -> Table {
    let key_style = Style::default().fg(Color::LightCyan);
    let help_style = Style::default().fg(Color::Gray);

    let mut rows = vec![];
    for action in actions.actions().iter() {
        let mut first = true;
        for key in action.keys() {
            let help = if first {
                first = false;
                action.to_string()
            } else {
                String::from("")
            };
            let row = Row::new(vec![
                Cell::from(Span::styled(key.to_string(), key_style)),
                Cell::from(Span::styled(help, help_style)),
            ]);
            rows.push(row);
        }
    }

    Table::new(rows)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Plain)
                .title("Help"),
        )
        .widths(&[Constraint::Length(11), Constraint::Min(20)])
        .column_spacing(1)
}

fn draw_logs<'a>() -> TuiLoggerWidget<'a> {
    TuiLoggerWidget::default()
        .style_error(Style::default().fg(Color::Red))
        .style_debug(Style::default().fg(Color::Green))
        .style_warn(Style::default().fg(Color::Yellow))
        .style_trace(Style::default().fg(Color::Gray))
        .style_info(Style::default().fg(Color::Blue))
        .block(
            Block::default()
                .title("Logs")
                .border_style(Style::default().fg(Color::White).bg(Color::Black))
                .borders(Borders::ALL),
        )
        .style(Style::default().fg(Color::White).bg(Color::Black))
}

fn draw_duration<'a>(duration: &Duration, total_duration: &Duration) -> LineGauge<'a> {
    let min = duration.as_secs() / 60;
    let sec = duration.as_secs() % 60;
    let label = format!("{}:{}", min, sec);

    let ms = duration.as_millis() as f64;
    let total_ms = total_duration.as_millis() as f64;
    let ratio = ms / total_ms;

    LineGauge::default()
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title("Sleep duration"),
        )
        .gauge_style(
            Style::default()
                .fg(Color::Cyan)
                .bg(Color::Black)
                .add_modifier(Modifier::BOLD),
        )
        .line_set(line::THICK)
        .label(label)
        .ratio(ratio)
}
