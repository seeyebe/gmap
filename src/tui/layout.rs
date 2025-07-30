use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::tui::state::{TuiState, WeekStats};

pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

pub fn get_visible_weeks<'a>(
    weeks: &'a [WeekStats],
    state: &TuiState,
    height: usize,
) -> Vec<(&'a WeekStats, bool)> {
    let view_height = height.saturating_sub(8);
    let filtered_weeks: Vec<_> = state
        .filtered_indices
        .iter()
        .filter_map(|&i| weeks.get(i))
        .collect();

    if filtered_weeks.is_empty() {
        return Vec::new();
    }

    let selected_in_filtered = state
        .filtered_indices
        .iter()
        .position(|&i| i == state.selected)
        .unwrap_or(0);

    let start = selected_in_filtered
        .saturating_sub(view_height / 2)
        .min(filtered_weeks.len().saturating_sub(view_height));
    let end = (start + view_height).min(filtered_weeks.len());

    filtered_weeks[start..end]
        .iter()
        .enumerate()
        .map(|(i, &week)| {
            let global_idx = state.filtered_indices[start + i];
            (week, global_idx == state.selected)
        })
        .collect()
}
