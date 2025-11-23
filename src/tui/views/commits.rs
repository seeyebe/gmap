use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState};
use ratatui::Frame;

use super::super::state::{TuiState, WeekStats};
use super::{header_cell, truncate};

/// Render the commit details view, including the commit list and the selected commit summary.
pub fn draw_commit_details_view(
    f: &mut Frame,
    area: Rect,
    weeks: &[WeekStats],
    state: &mut TuiState,
) {
    if weeks.is_empty() || state.selected >= weeks.len() {
        let placeholder = Paragraph::new("No week selected").block(
            Block::default()
                .title("Commit Details")
                .borders(Borders::ALL),
        );
        f.render_widget(placeholder, area);
        return;
    }

    let outer_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let main_area = outer_chunks[0];
    let selected_week = &weeks[state.selected];

    if state.loading_commits {
        let loading = Paragraph::new("Loading commits...").block(
            Block::default()
                .title(format!("Commit Details - Week {}", selected_week.week))
                .borders(Borders::ALL),
        );
        f.render_widget(loading, main_area);
        return;
    }

    if state.commit_details.is_empty() {
        let empty = Paragraph::new(
            "No commits found for this week.\nPress Enter from any other view to load commits.",
        )
        .block(
            Block::default()
                .title(format!("Commit Details - Week {}", selected_week.week))
                .borders(Borders::ALL),
        );
        f.render_widget(empty, main_area);
        return;
    }

    let inner_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
        .split(main_area);

    let indices: Vec<usize> =
        if !state.commit_search_query.is_empty() && !state.commit_filtered_indices.is_empty() {
            state.commit_filtered_indices.clone()
        } else {
            (0..state.commit_details.len()).collect()
        };

    if !indices.is_empty() && !indices.contains(&state.commit_selected) {
        state.commit_selected = indices[0];
    }

    let commit_rows: Vec<Row> = indices
        .iter()
        .map(|&i| {
            let commit = &state.commit_details[i];
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

            let message_cell = Cell::from(truncate(&commit.message, 50)).style(if is_selected {
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            });

            let author_cell =
                Cell::from(commit.author_name.clone()).style(Style::default().fg(Color::Magenta));

            let changes_cell =
                Cell::from(format!("+{} -{}", commit.lines_added, commit.lines_deleted))
                    .style(Style::default().fg(Color::Green));

            Row::new(vec![hash_cell, message_cell, author_cell, changes_cell])
        })
        .collect();

    let mut table_state = TableState::default();
    let pos_in_filtered = indices
        .iter()
        .position(|&i| i == state.commit_selected)
        .unwrap_or(0);
    table_state.select(Some(pos_in_filtered));

    let commits_table = Table::new(
        commit_rows,
        [
            Constraint::Length(10),
            Constraint::Percentage(50),
            Constraint::Percentage(25),
            Constraint::Length(15),
        ],
    )
    .header(Row::new([
        header_cell("Hash", Color::Yellow),
        header_cell("Message", Color::Yellow),
        header_cell("Author", Color::Yellow),
        header_cell("Changes", Color::Yellow),
    ]))
    .block(
        Block::default()
            .title(format!(
                "Commits - Week {} ({} commits)",
                selected_week.week,
                state.commit_details.len()
            ))
            .borders(Borders::ALL),
    );

    f.render_stateful_widget(commits_table, inner_chunks[0], &mut table_state);

    if let Some(selected_commit) = state.commit_details.get(state.commit_selected) {
        let details_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(8), Constraint::Min(0)])
            .split(inner_chunks[1]);

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

        let info_panel = Paragraph::new(commit_info).block(
            Block::default()
                .title("Info")
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        );
        f.render_widget(info_panel, details_chunks[0]);

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

        let files_panel = Paragraph::new(files_text).block(
            Block::default()
                .title(format!("Files ({})", selected_commit.files_changed.len()))
                .borders(Borders::ALL)
                .border_style(Style::default().fg(Color::Blue)),
        );
        f.render_widget(files_panel, details_chunks[1]);
    }
}
