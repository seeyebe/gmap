use crate::tui::state::{TuiState, WeekStats};
use ratatui::layout::{Constraint, Direction, Layout, Rect};

/// Return a centered rectangle of size `percent_x` × `percent_y` inside `r`.
pub fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let vertical = Layout::default()
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
        .split(vertical[1])[1]
}

/// Select the slice of weeks that should be visible, centering on the selected week when possible.
/// Returns pairs of (week, is_selected).
pub fn get_visible_weeks<'a>(
    weeks: &'a [WeekStats],
    state: &TuiState,
    height: usize,
) -> Vec<(&'a WeekStats, bool)> {
    if weeks.is_empty() {
        return Vec::new();
    }

    const VERTICAL_PADDING: usize = 8;
    let view_height = height.saturating_sub(VERTICAL_PADDING).max(1);

    let indices: Vec<usize> = if state.filtered_indices.is_empty() {
        (0..weeks.len()).collect()
    } else {
        state
            .filtered_indices
            .iter()
            .copied()
            .filter(|&i| i < weeks.len())
            .collect()
    };

    if indices.is_empty() {
        return Vec::new();
    }

    let selected_pos = indices
        .iter()
        .position(|&i| i == state.selected)
        .unwrap_or(0);
    let mut start = selected_pos.saturating_sub(view_height / 2);
    if start + view_height > indices.len() {
        start = indices.len().saturating_sub(view_height);
    }
    let end = (start + view_height).min(indices.len());

    indices[start..end]
        .iter()
        .map(|&global_idx| (&weeks[global_idx], global_idx == state.selected))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn week(idx: usize) -> WeekStats {
        WeekStats {
            week: format!("W{idx:02}"),
            commits: 1,
            lines_added: 1,
            lines_deleted: 0,
            top_authors: vec![],
            file_extensions: HashMap::new(),
            top_files: vec![],
        }
    }

    #[test]
    fn visible_weeks_center_on_selection() {
        let weeks: Vec<_> = (0..10).map(week).collect();
        let mut state = TuiState::default();
        state.selected = 5;
        state.filtered_indices = (0..weeks.len()).collect();

        let visible = get_visible_weeks(&weeks, &state, 12);

        assert_eq!(visible.len(), 4, "height minus padding drives view size");
        assert!(visible.iter().any(|(w, sel)| *sel && w.week == "W05"));
        assert_eq!(visible.first().unwrap().0.week, "W03");
    }

    #[test]
    fn visible_weeks_respect_filtered_indices() {
        let weeks: Vec<_> = (0..8).map(week).collect();
        let mut state = TuiState::default();
        state.selected = 3;
        state.filtered_indices = vec![1, 3, 5, 7];

        let visible = get_visible_weeks(&weeks, &state, 40);
        let weeks_only: Vec<_> = visible.into_iter().map(|(w, _)| w.week.clone()).collect();

        assert_eq!(weeks_only, vec!["W01", "W03", "W05", "W07"]);
    }
}
