use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, Borders, Cell, Row, Sparkline, Table};
use ratatui::Frame;

use super::super::state::{TuiState, WeekStats};

/// Render a simple commit timeline sparkline plus a table of recent weeks.
pub fn draw_timeline_view(f: &mut Frame, area: Rect, weeks: &[WeekStats], _state: &TuiState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    let commit_data: Vec<u64> = weeks.iter().map(|w| w.commits as u64).collect();

    if !commit_data.is_empty() {
        let commits_sparkline = Sparkline::default()
            .block(
                Block::default()
                    .title("Commits Over Time")
                    .borders(Borders::ALL),
            )
            .data(&commit_data)
            .style(Style::default().fg(ratatui::style::Color::Green));
        f.render_widget(commits_sparkline, chunks[0]);
    }

    let recent_weeks = weeks.iter().rev().take(10).collect::<Vec<_>>();
    let rows: Vec<Row> = recent_weeks
        .iter()
        .map(|week| {
            let week_cell = Cell::from(week.week.clone());
            let commits_cell = Cell::from(format!("{}", week.commits));
            let activity_level = if week.commits > 10 {
                "High"
            } else if week.commits > 5 {
                "Medium"
            } else if week.commits > 0 {
                "Low"
            } else {
                "Quiet"
            };
            let activity_cell = Cell::from(activity_level);

            Row::new(vec![week_cell, commits_cell, activity_cell])
        })
        .collect();

    let timeline_table = Table::new(
        rows,
        [
            Constraint::Percentage(40),
            Constraint::Percentage(30),
            Constraint::Percentage(30),
        ],
    )
    .header(Row::new([
        Cell::from("Week").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Commits").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Activity").style(Style::default().add_modifier(Modifier::BOLD)),
    ]))
    .block(
        Block::default()
            .title("Recent Activity Timeline")
            .borders(Borders::ALL),
    );

    f.render_widget(timeline_table, chunks[1]);
}
