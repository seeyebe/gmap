use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Gauge, Paragraph, Sparkline};
use ratatui::Frame;

use super::super::state::{TuiState, WeekStats};

/// Render the aggregate repository statistics view with gauges and a commit trend sparkline.
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
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Total Weeks: ", Style::default().fg(Color::White)),
            Span::styled(format!("{}", weeks.len()), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("Total Commits: ", Style::default().fg(Color::White)),
            Span::styled(
                format!("{total_commits}"),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::styled("Lines Added: ", Style::default().fg(Color::White)),
            Span::styled(format!("+{total_added}"), Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("Lines Deleted: ", Style::default().fg(Color::White)),
            Span::styled(format!("-{total_deleted}"), Style::default().fg(Color::Red)),
        ]),
        Line::from(vec![
            Span::styled("Net Change: ", Style::default().fg(Color::White)),
            Span::styled(
                format!("{net_change:+}"),
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
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![
            Span::styled("Average per week: ", Style::default().fg(Color::White)),
            Span::styled(format!("{avg_commits}"), Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled("Maximum week: ", Style::default().fg(Color::White)),
            Span::styled(format!("{max_commits}"), Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("Minimum week: ", Style::default().fg(Color::White)),
            Span::styled(format!("{min_commits}"), Style::default().fg(Color::Blue)),
        ]),
    ];

    let stats_para = Paragraph::new(stats_text).block(
        Block::default()
            .title("Overall Statistics")
            .borders(Borders::ALL),
    );
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
                selected_week.commits, max_commits, activity_ratio as u16
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
