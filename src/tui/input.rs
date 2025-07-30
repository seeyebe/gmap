use super::{WeekStats, TuiState};
use std::process::{Command, Stdio};
use std::io::Write;
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

pub fn copy_to_clipboard(text: &str) -> Result<(), String> {
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        if clipboard.set_text(text.to_string()).is_ok() {
            return Ok(());
        }
    }

    if Command::new("wl-copy").arg(text).output().is_ok() {
        return Ok(());
    }

    if let Ok(mut child) = Command::new("xclip")
        .args(["-selection", "clipboard"])
        .stdin(Stdio::piped())
        .spawn()
    {
        if let Some(stdin) = child.stdin.as_mut() {
            if stdin.write_all(text.as_bytes()).is_ok() && child.wait().is_ok() {
                return Ok(());
            }
        }
    }

    Err("Clipboard copy failed. Please install `wl-copy` (Wayland) or `xclip` (X11).".into())
}
