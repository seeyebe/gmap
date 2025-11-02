use super::{WeekStats, TuiState};
use std::io::Write;
use std::process::{Command, Stdio};

/// Update filtered_indices based on search_query, and ensure selection stays valid.
pub fn apply_search_filter(weeks: &[WeekStats], state: &mut TuiState) {
    if state.search_query.is_empty() {
        state.filtered_indices = (0..weeks.len()).collect();
    } else {
        let query = state.search_query.to_lowercase();
        state.filtered_indices = weeks
            .iter()
            .enumerate()
            .filter_map(|(i, week)| {
                if week.week.to_lowercase().contains(&query)
                    || week
                        .top_authors
                        .iter()
                        .any(|author| author.to_lowercase().contains(&query))
                {
                    Some(i)
                } else {
                    None
                }
            })
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
    // 1) macOS: pbcopy
    if let Ok(mut child) = Command::new("pbcopy")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        if let Some(stdin) = child.stdin.as_mut() {
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();
        return Ok(());
    }

    // 2) Windows: clip (falls back to powershell)
    if let Ok(mut child) = Command::new("clip")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        if let Some(stdin) = child.stdin.as_mut() {
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();
        return Ok(());
    } else if let Ok(mut child) = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            "Set-Clipboard -Value ([Console]::In.ReadToEnd())",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        if let Some(stdin) = child.stdin.as_mut() {
            let _ = stdin.write_all(text.as_bytes());
        }
        let _ = child.wait();
        return Ok(());
    }

    // 3) Wayland: wl-copy (persists in background)
    if let Ok(mut child) = Command::new("wl-copy")
        .arg("--type")
        .arg("text/plain")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        if let Some(stdin) = child.stdin.as_mut() {
            let _ = stdin.write_all(text.as_bytes());
        }
        // Do not wait — let wl-copy background to keep clipboard alive
        return Ok(());
    }

    // 4) X11: xclip (keep process alive by not waiting)
    if let Ok(mut child) = Command::new("xclip")
        .args(["-selection", "clipboard", "-in"])
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
    {
        if let Some(stdin) = child.stdin.as_mut() {
            let _ = stdin.write_all(text.as_bytes());
        }
        // Don't wait — xclip stays alive to own the selection
        return Ok(());
    }

    Err("Clipboard copy failed. Install one of: pbcopy (macOS), wl-copy (Wayland), xclip (X11), or use Windows clip.".into())
}

/// Update commit_filtered_indices based on commit_search_query.
pub fn apply_commit_search_filter(state: &mut TuiState) {
    if state.commit_search_query.is_empty() {
        state.commit_filtered_indices = (0..state.commit_details.len()).collect();
    } else {
        let q = state.commit_search_query.to_lowercase();
        state.commit_filtered_indices = state
            .commit_details
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                if c.message.to_lowercase().contains(&q)
                    || c.author_name.to_lowercase().contains(&q)
                    || c.short_hash.to_lowercase().contains(&q)
                {
                    Some(i)
                } else {
                    None
                }
            })
            .collect();
    }
    if state.commit_selected >= state.commit_filtered_indices.len() {
        state.commit_selected = state.commit_filtered_indices.len().saturating_sub(1);
    }
}
