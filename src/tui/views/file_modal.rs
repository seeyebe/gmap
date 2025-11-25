use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::tui::centered_rect;

use super::super::state::WeekStats;

/// Draw a popup showing top files by churn for the selected week.
pub fn draw_file_modal(f: &mut Frame, area: Rect, week: &WeekStats) {
    let popup = centered_rect(60, 60, area);
    f.render_widget(Clear, popup);

    let mut lines: Vec<Line> = Vec::new();
    lines.push(Line::from(vec![Span::styled(
        "File Explorer",
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD),
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
        lines.push(Line::from(format!("  {display_path} (+{changes} changes)")));
    }

    if week.top_files.len() > 10 {
        lines.push(Line::from(format!(
            "  … and {} more",
            week.top_files.len() - 10
        )));
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
