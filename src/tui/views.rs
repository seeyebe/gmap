use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Clear, Gauge, Paragraph, Row, Sparkline, Table};
use ratatui::Frame;

use crate::tui::centered_rect;

use super::{
    layout::{get_visible_weeks},
    draw::{enhanced_intensity_bar, get_intensity_color},
    state::{TuiState, WeekStats},
};

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
            let week_cell = if *is_selected {
                Cell::from(format!("{} ◄", week.week)).style(
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Cell::from(week.week.clone()).style(Style::default().fg(Color::White))
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
                "+{:>4}/-{:<4} ({}{})",
                week.lines_added,
                week.lines_deleted,
                if lines_delta >= 0 { "+" } else { "" },
                lines_delta
            ))
            .style(delta_style);

            let max_displayed = 3;
            let author_count = week.top_authors.len();
            let mut displayed = week.top_authors
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
        Cell::from("Week")
            .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Commits")
            .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Cell::from("Lines Changed")
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Cell::from("Top Authors")
            .style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
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
            Constraint::Length(8),
            Constraint::Length(6),
            Constraint::Min(0),
        ])
        .split(area);

    let week = &weeks[state.selected];
    let net_change = week.lines_added as i64 - week.lines_deleted as i64;
    let total_changes = week.lines_added + week.lines_deleted;

    let basic_stats = vec![
        Line::from(vec![Span::styled(
            "📅 Week Details",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
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

    let avg_commits = if !weeks.is_empty() {
        weeks.iter().map(|w| w.commits).sum::<usize>() / weeks.len()
    } else {
        0
    };

    let vs_avg = week.commits as i32 - avg_commits as i32;
    let comparison_text = vec![
        Line::from(vec![Span::styled(
            "📊 vs Repository Average",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
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
    let mut author_lines: Vec<Line> = week.top_authors
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
            Span::styled(format!("(+{} more)", author_count - max_displayed), Style::default().fg(Color::Gray)),
        ]));
    }

    let authors_text: Vec<Line> = vec![
        Line::from(vec![Span::styled(
            "👥 Top Contributors",
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
    ]
    .into_iter()
    .chain(author_lines)
    .collect();

    let authors_panel = Paragraph::new(authors_text).block(
        Block::default()
            .title("Contributors")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Blue)),
    );
    f.render_widget(authors_panel, chunks[2]);
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
    let avg_commits = if !weeks.is_empty() { total_commits / weeks.len() } else { 0 };
    let max_commits = weeks.iter().map(|w| w.commits).max().unwrap_or(0);
    let min_commits = weeks.iter().map(|w| w.commits).min().unwrap_or(0);

    let stats_text = vec![
        Line::from(vec![
            Span::styled("📊 Repository Statistics", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
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
            Span::styled(format!("{:+}", total_added as i64 - total_deleted as i64),
                        if total_added >= total_deleted { Style::default().fg(Color::Green) }
                        else { Style::default().fg(Color::Red) }),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("📈 Commit Statistics", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
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
        } else { 0.0 };

        let gauge = Gauge::default()
            .block(Block::default().title("Current Week Activity").borders(Borders::ALL))
            .gauge_style(Style::default().fg(Color::Green))
            .percent(activity_ratio as u16)
            .label(format!("{}/{} commits ({}%)", selected_week.commits, max_commits, activity_ratio as u16));
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

pub fn draw_filetypes_view(f: &mut Frame, area: Rect, weeks: &[WeekStats], state: &TuiState) {
    if weeks.is_empty() || state.selected >= weeks.len() {
        let placeholder = Paragraph::new("No data available")
            .block(Block::default().title("File Types Analysis").borders(Borders::ALL));
        f.render_widget(placeholder, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(50),
            Constraint::Percentage(50),
        ])
        .split(area);

    let selected_week = &weeks[state.selected];

    let mut extensions: Vec<_> = selected_week.file_extensions.iter().collect();
    extensions.sort_by(|a, b| b.1.commits.cmp(&a.1.commits));

    let ext_rows: Vec<Row> = extensions.iter().take(15).map(|(ext, stats)| {
        let ext_display = if ext.is_empty() { "no extension" } else { ext };
        Row::new(vec![
            Cell::from(ext_display.to_string()),
            Cell::from(format!("{}", stats.commits)),
            Cell::from(format!("+{}", stats.lines_added)),
            Cell::from(format!("-{}", stats.lines_deleted)),
            Cell::from(format!("{}", stats.files_changed)),
        ])
    }).collect();

    let extensions_table = Table::new(ext_rows, [
        Constraint::Length(12),
        Constraint::Length(8),
        Constraint::Length(8),
        Constraint::Length(8),
        Constraint::Length(8),
    ])
    .header(Row::new([
        Cell::from("Extension").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Commits").style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Cell::from("Added").style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Cell::from("Deleted").style(Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
        Cell::from("Files").style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    ]))
    .block(Block::default()
        .title(format!("File Extensions - Week {}", selected_week.week))
        .borders(Borders::ALL));

    f.render_widget(extensions_table, chunks[0]);

    let file_rows: Vec<Row> = selected_week.top_files.iter().take(15).map(|(path, changes)| {
        let display_path = if path.len() > 35 {
            format!("...{}", &path[path.len()-32..])
        } else {
            path.clone()
        };

        Row::new(vec![
            Cell::from(display_path),
            Cell::from(format!("{}", changes)),
        ])
    }).collect();

    let files_table = Table::new(file_rows, [
        Constraint::Percentage(80),
        Constraint::Percentage(20),
    ])
    .header(Row::new([
        Cell::from("File Path").style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        Cell::from("Changes").style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
    ]))
    .block(Block::default()
        .title("Most Changed Files")
        .borders(Borders::ALL));

    f.render_widget(files_table, chunks[1]);
}

pub fn draw_timeline_view(f: &mut Frame, area: Rect, weeks: &[WeekStats], _state: &TuiState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(60),
            Constraint::Percentage(40),
        ])
        .split(area);

    let commit_data: Vec<u64> = weeks.iter().map(|w| w.commits as u64).collect();
    let _added_data: Vec<u64> = weeks.iter().map(|w| w.lines_added as u64).collect();

    if !commit_data.is_empty() {
        let commits_sparkline = Sparkline::default()
            .block(Block::default().title("Commits Over Time").borders(Borders::ALL))
            .data(&commit_data)
            .style(Style::default().fg(Color::Green));
        f.render_widget(commits_sparkline, chunks[0]);
    }

    let recent_weeks = weeks.iter().rev().take(10).collect::<Vec<_>>();
    let rows: Vec<Row> = recent_weeks.iter().enumerate().map(|(_i, week)| {
        let week_cell = Cell::from(week.week.clone());
        let commits_cell = Cell::from(format!("{}", week.commits));
        let activity_level = if week.commits > 10 { "🔥 High" }
                           else if week.commits > 5 { "📈 Medium" }
                           else if week.commits > 0 { "📊 Low" }
                           else { "💤 Quiet" };
        let activity_cell = Cell::from(activity_level);

        Row::new(vec![week_cell, commits_cell, activity_cell])
    }).collect();

    let timeline_table = Table::new(rows, [
        Constraint::Percentage(40),
        Constraint::Percentage(30),
        Constraint::Percentage(30),
    ])
    .header(Row::new([
        Cell::from("Week").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Commits").style(Style::default().add_modifier(Modifier::BOLD)),
        Cell::from("Activity").style(Style::default().add_modifier(Modifier::BOLD)),
    ]))
    .block(Block::default().title("Recent Activity Timeline").borders(Borders::ALL));

    f.render_widget(timeline_table, chunks[1]);
}

pub fn draw_help_overlay(f: &mut Frame, area: Rect) {
    let block = Block::default().title("Help").borders(Borders::ALL);
    let help_area = centered_rect(60, 70, area);

    f.render_widget(Clear, help_area);

    let help_text = vec![
        Line::from(vec![Span::styled("Git Activity Heatmap - Help", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))]),
        Line::from(""),
        Line::from(vec![Span::styled("Navigation:", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))]),
        Line::from("  ← → j k     Navigate weeks"),
        Line::from("  Home/End    Jump to first/last week"),
        Line::from("  PgUp/PgDn   Navigate by 10 weeks"),
        Line::from(""),
        Line::from(vec![Span::styled("Views:", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))]),
        Line::from("  Tab         Next view mode"),
        Line::from("  Shift+Tab   Previous view mode"),
        Line::from(""),
        Line::from(vec![Span::styled("Search & Filter:", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))]),
        Line::from("  /           Start search"),
        Line::from("  Esc         Cancel search/Close help"),
        Line::from("  Enter       Apply search filter"),
        Line::from(""),
        Line::from(vec![Span::styled("General:", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))]),
        Line::from("  h, F1       Toggle this help"),
        Line::from("  q           Quit application"),
        Line::from(""),
        Line::from(vec![Span::styled("Views Available:", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
        Line::from("  🔥 Heatmap   - Weekly commit activity"),
        Line::from("  📊 Stats     - Repository statistics"),
        Line::from("  📁 Files     - File type analysis"),
        Line::from("  📈 Timeline  - Activity over time"),
        Line::from(""),
        Line::from(vec![Span::styled("Press 'h' or 'Esc' to close this help", Style::default().fg(Color::Gray))]),
    ];

    let help_paragraph = Paragraph::new(help_text)
        .block(block)
        .wrap(ratatui::widgets::Wrap { trim: true });
    f.render_widget(help_paragraph, help_area);
}