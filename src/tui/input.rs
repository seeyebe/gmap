use super::{WeekStats, TuiState};

pub fn apply_search_filter(weeks: &[WeekStats], state: &mut TuiState) {
    if state.search_query.is_empty() {
        state.filtered_indices = (0..weeks.len()).collect();
    } else {
        let query = state.search_query.to_lowercase();
        state.filtered_indices = weeks.iter()
            .enumerate()
            .filter(|(_, week)| {
                week.week.to_lowercase().contains(&query) ||
                week.top_authors.iter().any(|author| author.to_lowercase().contains(&query))
            })
            .map(|(i, _)| i)
            .collect();
    }

    ensure_selection_in_filtered(state);
}

pub fn ensure_selection_in_filtered(state: &mut TuiState) {
    if state.filtered_indices.is_empty() {
        return;
    }

    if !state.filtered_indices.contains(&state.selected) {
        state.selected = state.filtered_indices[0];
    }
}
