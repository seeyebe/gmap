use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, Borders, Cell, Clear, Gauge, Paragraph, Row, Sparkline, Table, TableState,
};
use ratatui::Frame;

use crate::tui::centered_rect;

use super::{
    layout::get_visible_weeks,
    draw::{enhanced_intensity_bar, get_intensity_color},
    state::{TuiState, WeekStats},
};

fn header_cell(text: &str, color: Color) -> Cell<'static> {
    Cell::from(text.to_string()).style(Style::default().fg(color).add_modifier(Modifier::BOLD))
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() > max { format!("{}...", &s[..max.saturating_sub(3)]) } else { s.to_string() }
}

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
            let commits_cell = Cell::from(format!("{:>3} {}", week.commits, intensity_bar))
                .style(commits_style);

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
                week.lines_added,
                week.lines_deleted,
                lines_delta
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
            let authors_cell = Cell::from(displayed.join(", "))
                .style(Style::default().fg(Color::Magenta));

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
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("Commits: ", Style::default().fg(Color::White)),
            Span::styled(format!("{}", week.commits), Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("Lines added: ", Style::default().fg(Color::White)),
            Span::styled(format!("+{}", week.lines_added), Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("Lines deleted: ", Style::default().fg(Color::White)),
            Span::styled(format!("-{}", week.lines_deleted), Style::default().fg(Color::Red)),
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
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("Repo average: ", Style::default().fg(Color::White)),
            Span::styled(format!("{}", avg_commits), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("Difference: ", Style::default().fg(Color::White)),
            Span::styled(
                format!("{:+}", vs_avg),
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
                Span::styled(format!("{} ", icon), Style::default().fg(Color::Yellow)),
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

    let mut authors_text: Vec<Line> = vec![
        Line::from(vec![Span::styled(
            "Top Contributors",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )]),
    ];
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
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
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
                Span::styled(format!("+{} ", changes), Style::default().fg(Color::Green)),
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

pub fn draw_statistics_view(f: &mut Frame, area: Rect, weeks: &[WeekStats], state: &TuiState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(30),
            Constraint::Percentage(40),
            Constraint::Percentage(30),
        ])
        .split(area);

    let total_commits: usize = weeks.iter().map(|w| w.commits).sum();
    let total_added: usize = weeks.iter().map(|w| w.lines_added).sum();
    let total_deleted: usize = weeks.iter().map(|w| w.lines_deleted).sum();
    let avg_commits = if !weeks.is_empty() {
        total_commits / weeks.len()
    } else {
        0
    };
    let max_commits = weeks.iter().map(|w| w.commits).max().unwrap_or(0);
    let min_commits = weeks.iter().map(|w| w.commits).min().unwrap_or(0);

    let net_change = total_added as i64 - total_deleted as i64;

    let stats_text = vec![
        Line::from(vec![Span::styled(
            "Repository Statistics",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Total Weeks: ", Style::default().fg(Color::White)),
            Span::styled(format!("{}", weeks.len()), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("Total Commits: ", Style::default().fg(Color::White)),
            Span::styled(format!("{}", total_commits), Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("Lines Added: ", Style::default().fg(Color::White)),
            Span::styled(format!("+{}", total_added), Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("Lines Deleted: ", Style::default().fg(Color::White)),
            Span::styled(format!("-{}", total_deleted), Style::default().fg(Color::Red)),
        ]),
        Line::from(vec![
            Span::styled("Net Change: ", Style::default().fg(Color::White)),
            Span::styled(
                format!("{:+}", net_change),
                if net_change >= 0 {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::Red)
                },
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "📈 Commit Statistics",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("Average per week: ", Style::default().fg(Color::White)),
            Span::styled(format!("{}", avg_commits), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("Maximum week: ", Style::default().fg(Color::White)),
            Span::styled(format!("{}", max_commits), Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("Minimum week: ", Style::default().fg(Color::White)),
            Span::styled(format!("{}", min_commits), Style::default().fg(Color::Blue)),
        ]),
    ];

    let stats_para = Paragraph::new(stats_text)
        .block(Block::default().title("Overall Statistics").borders(Borders::ALL));
    f.render_widget(stats_para, chunks[0]);

    if !weeks.is_empty() && state.selected < weeks.len() {
        let selected_week = &weeks[state.selected];
        let activity_ratio = if max_commits > 0 {
            (selected_week.commits as f64 / max_commits as f64) * 100.0
        } else {
            0.0
        };

        let gauge = Gauge::default()
            .block(
                Block::default()
                    .title("Current Week Activity")
                    .borders(Borders::ALL),
            )
            .gauge_style(Style::default().fg(Color::Green))
            .percent(activity_ratio as u16)
            .label(format!(
                "{}/{} commits ({}%)",
                selected_week.commits,
                max_commits,
                activity_ratio as u16
            ));
        f.render_widget(gauge, chunks[1]);
    }

    let trend_data: Vec<u64> = weeks.iter().map(|w| w.commits as u64).collect();
    if trend_data.len() > 1 {
        let sparkline = Sparkline::default()
            .block(Block::default().title("Commit Trend").borders(Borders::ALL))
            .data(&trend_data)
            .style(Style::default().fg(Color::Yellow));
        f.render_widget(sparkline, chunks[2]);
    }
}

pub fn draw_files_view(f: &mut Frame, area: Rect, weeks: &[WeekStats], state: &TuiState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let mut overall: std::collections::HashMap<String, (usize, usize, usize, usize)> = std::collections::HashMap::new();
    for w in weeks {
        for (ext, s) in &w.file_extensions {
            let e = overall.entry(ext.clone()).or_insert((0, 0, 0, 0));
            e.0 += s.commits;
            e.1 += s.files_changed;
            e.2 += s.lines_added;
            e.3 += s.lines_deleted;
        }
    }

    let mut overall_vec: Vec<(String, usize, usize, usize, usize)> = overall
        .into_iter()
        .map(|(ext, v)| (ext, v.0, v.1, v.2, v.3))
        .collect();
    overall_vec.sort_by(|a, b| b.3.cmp(&a.3));

    let overall_rows: Vec<Row> = overall_vec
        .into_iter()
        .map(|(ext, commits, files, added, deleted)| Row::new(vec![
            Cell::from(if ext.is_empty() { "(none)".to_string() } else { ext }),
            Cell::from(format!("{}", commits)),
            Cell::from(format!("{}", files)),
            Cell::from(format!("+{}", added)).style(Style::default().fg(Color::Green)),
            Cell::from(format!("-{}", deleted)).style(Style::default().fg(Color::Red)),
        ]))
        .collect();

    let overall_table = Table::new(
        overall_rows,
        [
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(12),
        ],
    )
    .header(Row::new([
        header_cell("Ext", Color::Yellow),
        header_cell("Commits", Color::Green),
        header_cell("Files", Color::Cyan),
        header_cell("Added", Color::Green),
        header_cell("Deleted", Color::Red),
    ]))
    .block(Block::default().title("Overall File Types").borders(Borders::ALL));

    f.render_widget(overall_table, chunks[1]);

    if weeks.is_empty() || state.selected >= weeks.len() {
        let placeholder = Paragraph::new("No data").block(Block::default().title("Selected Week").borders(Borders::ALL));
        f.render_widget(placeholder, chunks[0]);
        return;
    }

    let w = &weeks[state.selected];
    let mut week_vec: Vec<(&String, &super::super::heat::FileExtensionStats)> =
        w.file_extensions.iter().collect();
    week_vec.sort_by(|a, b| b.1.lines_added.cmp(&a.1.lines_added));
    let week_rows: Vec<Row> = week_vec
        .into_iter()
        .map(|(ext, s)| Row::new(vec![
            Cell::from(if ext.is_empty() { "(none)".to_string() } else { ext.clone() }),
            Cell::from(format!("{}", s.commits)),
            Cell::from(format!("{}", s.files_changed)),
            Cell::from(format!("+{}", s.lines_added)).style(Style::default().fg(Color::Green)),
            Cell::from(format!("-{}", s.lines_deleted)).style(Style::default().fg(Color::Red)),
        ]))
        .collect();

    let week_table = Table::new(
        week_rows,
        [
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(12),
        ],
    )
    .header(Row::new([
        header_cell("Ext", Color::Yellow),
        header_cell("Commits", Color::Green),
        header_cell("Files", Color::Cyan),
        header_cell("Added", Color::Green),
        header_cell("Deleted", Color::Red),
    ]))
    .block(Block::default().title(format!("File Types - {}", w.week)).borders(Borders::ALL));

    f.render_widget(week_table, chunks[0]);
}

pub fn draw_dashboard(f: &mut Frame, area: Rect, weeks: &[WeekStats], state: &TuiState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(36), Constraint::Percentage(34), Constraint::Percentage(30)])
        .split(area);

    // Left: periods list (reusing heatmap rows but compact)
    let visible_weeks = get_visible_weeks(weeks, state, f.size().height as usize);
    let max_commits = weeks.iter().map(|ws| ws.commits).max().unwrap_or(1);

    let rows: Vec<Row> = visible_weeks
        .iter()
        .map(|(week, is_selected)| {
            let intensity_bar = enhanced_intensity_bar(week.commits, max_commits);
            let week_label = if *is_selected { format!("{} ◄", week.week) } else { week.week.clone() };
            let week_cell = if *is_selected {
                Cell::from(week_label).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            } else {
                Cell::from(week_label).style(Style::default().fg(Color::White))
            };
            let commits_style = get_intensity_color(week.commits, max_commits);
            let commits_cell = Cell::from(format!("{:>3} {}", week.commits, intensity_bar)).style(commits_style);
            let lines_delta = week.lines_added as i64 - week.lines_deleted as i64;
            let delta_style = if lines_delta > 0 { Style::default().fg(Color::Green) } else if lines_delta < 0 { Style::default().fg(Color::Red) } else { Style::default().fg(Color::White) };
            let sign = if lines_delta >= 0 { "+" } else { "" };
            let lines_cell = Cell::from(format!("{}{}", sign, lines_delta)).style(delta_style);
            Row::new(vec![week_cell, commits_cell, lines_cell])
        })
        .collect();

    let periods = Table::new(rows, [Constraint::Length(12), Constraint::Length(12), Constraint::Length(10)])
        .header(Row::new([
            header_cell("Period", Color::Yellow),
            header_cell("Commits", Color::Green),
            header_cell("Δlines", Color::Cyan),
        ]))
        .block(Block::default().title("Periods").borders(Borders::ALL));
    f.render_widget(periods, chunks[0]);

    // Middle: commit list (filtered)
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
                    Cell::from(format!("{} ◄", commit.short_hash)).style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                } else {
                    Cell::from(commit.short_hash.clone()).style(Style::default().fg(Color::Cyan))
                };
                let message_cell = Cell::from(truncate(&commit.message, 50)).style(
                    if is_selected { Style::default().fg(Color::White).add_modifier(Modifier::BOLD) } else { Style::default().fg(Color::White) },
                );
                let author_cell = Cell::from(commit.author_name.clone()).style(Style::default().fg(Color::Magenta));
                Row::new(vec![hash_cell, message_cell, author_cell])
            })
            .collect()
    };
    let mut table_state = ratatui::widgets::TableState::default();
    table_state.select(Some(state.commit_selected));
    let commits_table = Table::new(commit_rows, [Constraint::Length(10), Constraint::Percentage(60), Constraint::Percentage(30)])
        .header(Row::new([
            header_cell("Hash", Color::Yellow),
            header_cell("Message", Color::Yellow),
            header_cell("Author", Color::Yellow),
        ]))
        .block(Block::default().title("Commits").borders(Borders::ALL));
    f.render_stateful_widget(commits_table, chunks[1], &mut table_state);

    // Right: commit details
    if let Some(selected_commit) = state.commit_details.get(state.commit_selected) {
        let details_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(8), Constraint::Min(0)])
            .split(chunks[2]);

        let commit_info = vec![
            Line::from(vec![Span::styled("Commit Details", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))]),
            Line::from(""),
            Line::from(vec![Span::styled("Hash: ", Style::default().fg(Color::White)), Span::styled(selected_commit.short_hash.clone(), Style::default().fg(Color::Cyan))]),
            Line::from(vec![Span::styled("Author: ", Style::default().fg(Color::White)), Span::styled(selected_commit.author_name.clone(), Style::default().fg(Color::Magenta))]),
            Line::from(vec![Span::styled("Date: ", Style::default().fg(Color::White)), Span::styled(selected_commit.timestamp.format("%Y-%m-%d %H:%M:%S").to_string(), Style::default().fg(Color::Green))]),
            Line::from(vec![Span::styled("Changes: ", Style::default().fg(Color::White)), Span::styled(format!("+{} -{}", selected_commit.lines_added, selected_commit.lines_deleted), Style::default().fg(Color::Green))]),
        ];
        f.render_widget(Paragraph::new(commit_info).block(Block::default().title("Info").borders(Borders::ALL)), details_chunks[0]);

        let files_text: Vec<Line> = std::iter::once(Line::from(vec![Span::styled("Files Changed", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))]))
            .chain(std::iter::once(Line::from("")))
            .chain(selected_commit.files_changed.iter().take(20).map(|file| {
                let display_path = if file.len() > 40 { format!("...{}", &file[file.len() - 37..]) } else { file.clone() };
                Line::from(vec![Span::raw("  "), Span::styled(display_path, Style::default().fg(Color::Cyan))])
            }))
            .collect();
        f.render_widget(Paragraph::new(files_text).block(Block::default().title("Files").borders(Borders::ALL)), details_chunks[1]);
    } else {
        f.render_widget(Paragraph::new("No commit selected").block(Block::default().title("Details").borders(Borders::ALL)), chunks[2]);
    }
}


pub fn draw_timeline_view(f: &mut Frame, area: Rect, weeks: &[WeekStats], _state: &TuiState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    let commit_data: Vec<u64> = weeks.iter().map(|w| w.commits as u64).collect();

    if !commit_data.is_empty() {
        let commits_sparkline = Sparkline::default()
            .block(Block::default().title("Commits Over Time").borders(Borders::ALL))
            .data(&commit_data)
            .style(Style::default().fg(Color::Green));
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
    .block(Block::default().title("Recent Activity Timeline").borders(Borders::ALL));

    f.render_widget(timeline_table, chunks[1]);
}

pub fn draw_commit_details_view(f: &mut Frame, area: Rect, weeks: &[WeekStats], state: &mut TuiState) {
    if weeks.is_empty() || state.selected >= weeks.len() {
        let placeholder = Paragraph::new("No week selected")
            .block(Block::default().title("Commit Details").borders(Borders::ALL));
        f.render_widget(placeholder, area);
        return;
    }

    let outer_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let main_area = outer_chunks[0];
    let _status_area = outer_chunks[1];

    let selected_week = &weeks[state.selected];

    if state.loading_commits {
        let loading = Paragraph::new("Loading commits...")
            .block(
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

    let indices: Vec<usize> = if !state.commit_search_query.is_empty() && !state.commit_filtered_indices.is_empty() {
        state.commit_filtered_indices.clone()
    } else {
        (0..state.commit_details.len()).collect()
    };

    // Ensure selection refers to a valid index; if not, pick first
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

            let message_cell = Cell::from(truncate(&commit.message, 50)).style(
                if is_selected {
                    Style::default().fg(Color::White).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                },
            );

            let author_cell =
                Cell::from(commit.author_name.clone()).style(Style::default().fg(Color::Magenta));

            let changes_cell = Cell::from(format!("+{} -{}", commit.lines_added, commit.lines_deleted))
                .style(Style::default().fg(Color::Green));

            Row::new(vec![hash_cell, message_cell, author_cell, changes_cell])
        })
        .collect();

    let mut table_state = TableState::default();
    let pos_in_filtered = indices.iter().position(|&i| i == state.commit_selected).unwrap_or(0);
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
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
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
                    selected_commit.timestamp.format("%Y-%m-%d %H:%M:%S").to_string(),
                    Style::default().fg(Color::Green),
                ),
            ]),
            Line::from(vec![
                Span::styled("Changes: ", Style::default().fg(Color::White)),
                Span::styled(
                    format!("+{} -{}", selected_commit.lines_added, selected_commit.lines_deleted),
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
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )]))
        .chain(std::iter::once(Line::from("")))
        .chain(
            selected_commit
                .files_changed
                .iter()
                .take(20)
                .map(|file| {
                    let display_path = if file.len() > 40 {
                        format!("...{}", &file[file.len() - 37..])
                    } else {
                        file.clone()
                    };
                    Line::from(vec![
                        Span::raw("  "),
                        Span::styled(display_path, Style::default().fg(Color::Cyan)),
                    ])
                }),
        )
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

pub fn draw_help_overlay(f: &mut Frame, area: Rect) {
    let block = Block::default().title("Help").borders(Borders::ALL);
    let help_area = centered_rect(70, 80, area);

    f.render_widget(Clear, help_area);

    let help_text = vec![
        Line::from(vec![Span::styled(
            "gmap - Help",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Navigation:",
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )]),
        Line::from("  j/k or ↑/↓  Move selection"),
        Line::from("  g/G         Jump to first/last"),
        Line::from("  PgUp/PgDn   Move by 10 items"),
        Line::from("  Mouse       Scroll with wheel"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Views:",
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )]),
        Line::from("  Tab         Next view (Heatmap/Stats/Timeline/Commits)"),
        Line::from("  Shift+Tab   Previous view"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Actions:",
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )]),
        Line::from("  c / y       Copy full / short hash"),
        Line::from("  o           Open commit in pager (git show)"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Search & Filter:",
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )]),
        Line::from("  /           Filter periods"),
        Line::from("  :           Filter commits (message/author/hash)"),
        Line::from("  p           Set path prefix filter"),
        Line::from("  m/M         Toggle monthly/include merges"),
        Line::from("  A           Toggle show-all vs last 12m/52w"),
        Line::from("  Esc         Cancel input / close help"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "General:",
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )]),
        Line::from("  h, F1       Toggle this help"),
        Line::from("  q           Quit application"),
        Line::from(""),
        // no tabs/views here; single dashboard
        Line::from(""),
        Line::from(vec![Span::styled(
            "Press 'h' or 'Esc' to close this help",
            Style::default().fg(Color::Gray),
        )]),
    ];

    let help_paragraph = Paragraph::new(help_text)
        .block(block)
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(help_paragraph, help_area);
}

pub fn draw_file_modal(f: &mut Frame, area: Rect, week: &WeekStats) {
    let popup = centered_rect(60, 60, area);
    f.render_widget(Clear, popup);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![Span::styled(
        "File Explorer",
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
    )]));
    lines.push(Line::from(format!("Week: {}", week.week)));
    lines.push(Line::from(""));

    lines.push(Line::from(vec![Span::styled(
        "Top files by churn:",
        Style::default().add_modifier(Modifier::BOLD),
    )]));
    for (path, changes) in week.top_files.iter().take(10) {
        let display_path = if path.len() > 50 {
            format!("...{}", &path[path.len() - 47..])
        } else {
            path.clone()
        };
        lines.push(Line::from(format!("  {} (+{} changes)", display_path, changes)));
    }

    if week.top_files.len() > 10 {
        lines.push(Line::from(format!("  … and {} more", week.top_files.len() - 10)));
    }

    lines.push(Line::from(""));
    lines.push(Line::from("Press Esc to close"));

    let paragraph = Paragraph::new(lines).block(
        Block::default()
            .title("File Drill-down")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );
    f.render_widget(paragraph, popup);
}
