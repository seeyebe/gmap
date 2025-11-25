use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

use crate::tui::centered_rect;

/// Draw the modal help overlay describing navigation, views, and shortcuts.
pub fn draw_help_overlay(f: &mut Frame, area: Rect) {
    let block = Block::default().title("Help").borders(Borders::ALL);
    let help_area = centered_rect(70, 80, area);

    f.render_widget(Clear, help_area);

    let help_text = vec![
        Line::from(vec![Span::styled(
            "gmap - Help",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Navigation:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from("  j/k or ↑/↓  Move selection"),
        Line::from("  g/G         Jump to first/last"),
        Line::from("  PgUp/PgDn   Move by 10 items"),
        Line::from("  Mouse       Scroll with wheel"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Views:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from("  Tab         Next view (Heatmap/Stats/Timeline/Commits)"),
        Line::from("  Shift+Tab   Previous view"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Actions:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from("  c / y       Copy full / short hash"),
        Line::from("  o           Open commit in pager (git show)"),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Search & Filter:",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
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
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from("  h, F1       Toggle this help"),
        Line::from("  q           Quit application"),
        Line::from(""),
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
