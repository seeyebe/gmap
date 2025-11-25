use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::Cell;

mod commits;
mod dashboard;
mod file_modal;
mod files;
mod heatmap;
mod help;
mod statistics;
mod timeline;

pub use commits::draw_commit_details_view;
pub use dashboard::draw_dashboard;
pub use file_modal::draw_file_modal;
pub use files::draw_files_view;
pub use heatmap::{draw_enhanced_side_panel, draw_heatmap_view};
pub use help::draw_help_overlay;
pub use statistics::draw_statistics_view;
pub use timeline::draw_timeline_view;

/// Convenience helper to build a styled table header cell.
pub(crate) fn header_cell(text: &str, color: Color) -> Cell<'static> {
    Cell::from(text.to_string()).style(Style::default().fg(color).add_modifier(Modifier::BOLD))
}

/// Truncate a string to `max` chars with an ellipsis when necessary.
pub(crate) fn truncate(s: &str, max: usize) -> String {
    if s.len() > max {
        format!("{}...", &s[..max.saturating_sub(3)])
    } else {
        s.to_string()
    }
}
