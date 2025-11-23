use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use ratatui::Frame;

use super::super::{
    draw::{enhanced_intensity_bar, get_intensity_color},
    layout::get_visible_weeks,
    state::{TuiState, WeekStats},
};
use super::header_cell;

/// Render the heatmap view showing weekly activity and a side panel of details.
pub fn draw_heatmap_view(f: &mut Frame, area: Rect, weeks: &[WeekStats], state: &TuiState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
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
            let lines_cell = Cell::from(format!(
                "+{:>4}/-{:<4} ({:+})",
                week.lines_added, week.lines_deleted, lines_delta
            ))
            .style(delta_style);

            let max_displayed = 3;
            let author_count = week.top_authors.len();
            let mut displayed = week
                .top_authors
                .iter()
                .take(max_displayed)
                .cloned()
                .collect::<Vec<_>>();
            if author_count > max_displayed {
                displayed.push(format!("… (+{} more)", author_count - max_displayed));
            }
            let authors_cell =
                Cell::from(displayed.join(", ")).style(Style::default().fg(Color::Magenta));

            Row::new(vec![week_cell, commits_cell, lines_cell, authors_cell])
        })
        .collect();

    let title = if state.search_mode {
        format!(
            "Heatmap: Git Activity | Search: {} | Press Esc to cancel",
            state.search_query
        )
    } else if !state.search_query.is_empty() {
        format!(
            "Heatmap: Git Activity | Filtered: '{}' ({} results)",
            state.search_query,
            state.filtered_indices.len()
        )
    } else {
        "Heatmap: Git Activity | Press 'h' for help, '/' to search".to_string()
    };

    let table = Table::new(
        rows,
        [
            Constraint::Length(12),
            Constraint::Length(16),
            Constraint::Length(24),
            Constraint::Percentage(100),
        ],
    )
    .header(Row::new([
        header_cell("Week", Color::Yellow),
        header_cell("Commits", Color::Green),
        header_cell("Lines Changed", Color::Cyan),
        header_cell("Top Authors", Color::Magenta),
    ]))
    .block(
        Block::default()
            .title(title)
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );

    f.render_widget(table, chunks[0]);
    draw_enhanced_side_panel(f, chunks[1], weeks, state);
}

/// Render the right-hand summary panel with week stats, comparisons, authors, and top files.
pub fn draw_enhanced_side_panel(f: &mut Frame, area: Rect, weeks: &[WeekStats], state: &TuiState) {
    if weeks.is_empty() || state.selected >= weeks.len() {
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(6),
            Constraint::Length(6),
            Constraint::Length(7),
            Constraint::Min(0),
        ])
        .split(area);

    let week = &weeks[state.selected];
    let net_change = week.lines_added as i64 - week.lines_deleted as i64;
    let total_changes = week.lines_added + week.lines_deleted;

    let basic_stats = vec![
        Line::from(vec![Span::styled(
            "Week Details",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("Commits: ", Style::default().fg(Color::White)),
            Span::styled(
                format!("{}", week.commits),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::styled("Lines added: ", Style::default().fg(Color::White)),
            Span::styled(
                format!("+{}", week.lines_added),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::styled("Lines deleted: ", Style::default().fg(Color::White)),
            Span::styled(
                format!("-{}", week.lines_deleted),
                Style::default().fg(Color::Red),
            ),
        ]),
        Line::from(vec![
            Span::styled("Net change: ", Style::default().fg(Color::White)),
            Span::styled(
                format!("{net_change:+}"),
                if net_change >= 0 {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Red)
                },
            ),
        ]),
        Line::from(vec![
            Span::styled("Total changes: ", Style::default().fg(Color::White)),
            Span::styled(format!("{total_changes}"), Style::default().fg(Color::Cyan)),
        ]),
    ];

    let basic_panel = Paragraph::new(basic_stats).block(
        Block::default()
            .title("Week Stats")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );
    f.render_widget(basic_panel, chunks[0]);

    let avg_commits = weeks.iter().map(|w| w.commits).sum::<usize>() / weeks.len().max(1);
    let vs_avg = week.commits as i32 - avg_commits as i32;

    let comparison_text = vec![
        Line::from(vec![Span::styled(
            "vs Repository Average",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("Repo average: ", Style::default().fg(Color::White)),
            Span::styled(format!("{avg_commits}"), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("Difference: ", Style::default().fg(Color::White)),
            Span::styled(
                format!("{vs_avg:+}"),
                if vs_avg >= 0 {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Red)
                },
            ),
        ]),
    ];

    let comparison_panel = Paragraph::new(comparison_text).block(
        Block::default()
            .title("Comparison")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );
    f.render_widget(comparison_panel, chunks[1]);

    let max_displayed = 5;
    let author_count = week.top_authors.len();
    let mut author_lines: Vec<Line> = week
        .top_authors
        .iter()
        .take(max_displayed)
        .enumerate()
        .map(|(i, author)| {
            let icon = match i {
                0 => "🥇",
                1 => "🥈",
                2 => "🥉",
                _ => "👤",
            };
            Line::from(vec![
                Span::styled(format!("{icon} "), Style::default().fg(Color::Yellow)),
                Span::styled(author.clone(), Style::default().fg(Color::Magenta)),
            ])
        })
        .collect();

    if author_count > max_displayed {
        author_lines.push(Line::from(vec![
            Span::raw("… "),
            Span::styled(
                format!("(+{} more)", author_count - max_displayed),
                Style::default().fg(Color::Gray),
            ),
        ]));
    }

    let mut authors_text: Vec<Line> = vec![Line::from(vec![Span::styled(
        "Top Contributors",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
    )])];
    authors_text.extend(author_lines);

    let authors_panel = Paragraph::new(authors_text).block(
        Block::default()
            .title("Contributors")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );
    f.render_widget(authors_panel, chunks[2]);

    let top_files_display: Vec<Line> = {
        let mut lines = vec![
            Line::from(vec![Span::styled(
                "Top Files This Week",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(""),
        ];

        for (path, changes) in week.top_files.iter().take(3) {
            let short_path = if path.len() > 25 {
                format!("...{}", &path[path.len() - 22..])
            } else {
                path.clone()
            };
            let base = (week.lines_added + week.lines_deleted).max(1);
            let bar = enhanced_intensity_bar(*changes, base);
            lines.push(Line::from(vec![
                Span::styled(short_path, Style::default().fg(Color::Cyan)),
                Span::raw(" "),
                Span::styled(format!("+{changes} "), Style::default().fg(Color::Green)),
                Span::styled(bar, Style::default().fg(Color::Magenta)),
            ]));
        }

        if week.top_files.len() > 3 {
            lines.push(Line::from(vec![
                Span::raw("… "),
                Span::styled(
                    format!("(+{} more)", week.top_files.len() - 3),
                    Style::default().fg(Color::Gray),
                ),
            ]));
        }

        lines
    };

    let files_summary_panel = Paragraph::new(top_files_display).block(
        Block::default()
            .title("Top Files")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );
    f.render_widget(files_summary_panel, chunks[3]);
}
