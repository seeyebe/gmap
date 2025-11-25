use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};
use ratatui::Frame;

use super::super::draw::{enhanced_intensity_bar, get_intensity_color};
use super::super::layout::get_visible_weeks;
use super::super::state::{TuiState, WeekStats};
use super::{header_cell, truncate};

/// Render the composite dashboard view combining periods, commit list, and details.
pub fn draw_dashboard(f: &mut Frame, area: Rect, weeks: &[WeekStats], state: &TuiState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(36),
            Constraint::Percentage(34),
            Constraint::Percentage(30),
        ])
        .split(area);

    let visible_weeks = get_visible_weeks(weeks, state, f.size().height as usize);
    let max_commits = weeks.iter().map(|ws| ws.commits).max().unwrap_or(1);

    let rows: Vec<Row> = visible_weeks
        .iter()
        .map(|(week, is_selected)| {
            let intensity_bar = enhanced_intensity_bar(week.commits, max_commits);
            let week_label = if *is_selected {
                format!("{} ◄", week.week)
            } else {
                week.week.clone()
            };
            let week_cell = if *is_selected {
                Cell::from(week_label).style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Cell::from(week_label).style(Style::default().fg(Color::White))
            };
            let commits_style = get_intensity_color(week.commits, max_commits);
            let commits_cell =
                Cell::from(format!("{:>3} {}", week.commits, intensity_bar)).style(commits_style);
            let lines_delta = week.lines_added as i64 - week.lines_deleted as i64;
            let delta_style = if lines_delta > 0 {
                Style::default().fg(Color::Green)
            } else if lines_delta < 0 {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::White)
            };
            let sign = if lines_delta >= 0 { "+" } else { "" };
            let lines_cell = Cell::from(format!("{sign}{lines_delta}")).style(delta_style);
            Row::new(vec![week_cell, commits_cell, lines_cell])
        })
        .collect();

    let periods = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(10),
        ],
    )
    .header(Row::new([
        header_cell("Period", Color::Yellow),
        header_cell("Commits", Color::Green),
        header_cell("Δlines", Color::Cyan),
    ]))
    .block(Block::default().title("Periods").borders(Borders::ALL));
    f.render_widget(periods, chunks[0]);

    let commit_rows: Vec<Row> = {
        let indices = if state.commit_filtered_indices.is_empty() {
            (0..state.commit_details.len()).collect::<Vec<_>>()
        } else {
            state.commit_filtered_indices.clone()
        };
        indices
            .into_iter()
            .map(|i| (i, &state.commit_details[i]))
            .map(|(i, commit)| {
                let is_selected = i == state.commit_selected;
                let hash_cell = if is_selected {
                    Cell::from(format!("{} ◄", commit.short_hash)).style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                } else {
                    Cell::from(commit.short_hash.clone()).style(Style::default().fg(Color::Cyan))
                };
                let message_cell =
                    Cell::from(truncate(&commit.message, 50)).style(if is_selected {
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    });
                let author_cell = Cell::from(commit.author_name.clone())
                    .style(Style::default().fg(Color::Magenta));
                Row::new(vec![hash_cell, message_cell, author_cell])
            })
            .collect()
    };
    let mut table_state = TableState::default();
    table_state.select(Some(state.commit_selected));
    let commits_table = Table::new(
        commit_rows,
        [
            Constraint::Length(10),
            Constraint::Percentage(60),
            Constraint::Percentage(30),
        ],
    )
    .header(Row::new([
        header_cell("Hash", Color::Yellow),
        header_cell("Message", Color::Yellow),
        header_cell("Author", Color::Yellow),
    ]))
    .block(Block::default().title("Commits").borders(Borders::ALL));
    f.render_stateful_widget(commits_table, chunks[1], &mut table_state);

    if let Some(selected_commit) = state.commit_details.get(state.commit_selected) {
        let details_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(8), Constraint::Min(0)])
            .split(chunks[2]);

        let commit_info = vec![
            Line::from(vec![Span::styled(
                "Commit Details",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
            Line::from(vec![
                Span::styled("Hash: ", Style::default().fg(Color::White)),
                Span::styled(
                    selected_commit.short_hash.clone(),
                    Style::default().fg(Color::Cyan),
                ),
            ]),
            Line::from(vec![
                Span::styled("Author: ", Style::default().fg(Color::White)),
                Span::styled(
                    selected_commit.author_name.clone(),
                    Style::default().fg(Color::Magenta),
                ),
            ]),
            Line::from(vec![
                Span::styled("Date: ", Style::default().fg(Color::White)),
                Span::styled(
                    selected_commit
                        .timestamp
                        .format("%Y-%m-%d %H:%M:%S")
                        .to_string(),
                    Style::default().fg(Color::Green),
                ),
            ]),
            Line::from(vec![
                Span::styled("Changes: ", Style::default().fg(Color::White)),
                Span::styled(
                    format!(
                        "+{} -{}",
                        selected_commit.lines_added, selected_commit.lines_deleted
                    ),
                    Style::default().fg(Color::Green),
                ),
            ]),
        ];
        f.render_widget(
            Paragraph::new(commit_info).block(Block::default().title("Info").borders(Borders::ALL)),
            details_chunks[0],
        );

        let files_text: Vec<Line> = std::iter::once(Line::from(vec![Span::styled(
            "Files Changed",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]))
        .chain(std::iter::once(Line::from("")))
        .chain(selected_commit.files_changed.iter().take(20).map(|file| {
            let display_path = if file.len() > 40 {
                format!("...{}", &file[file.len() - 37..])
            } else {
                file.clone()
            };
            Line::from(vec![
                Span::raw("  "),
                Span::styled(display_path, Style::default().fg(Color::Cyan)),
            ])
        }))
        .collect();
        f.render_widget(
            Paragraph::new(files_text).block(Block::default().title("Files").borders(Borders::ALL)),
            details_chunks[1],
        );
    } else {
        f.render_widget(
            Paragraph::new("No commit selected")
                .block(Block::default().title("Details").borders(Borders::ALL)),
            chunks[2],
        );
    }
}
