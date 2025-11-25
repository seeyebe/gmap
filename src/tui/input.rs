use super::{TuiState, WeekStats};
use std::io::Write;
use std::process::{Command, Stdio};

/// Update `filtered_indices` based on `search_query`, and ensure selection stays valid.
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

/// Keep the current selection inside the filtered list, defaulting to the first match.
pub fn ensure_selection_in_filtered(state: &mut TuiState) {
    if state.filtered_indices.is_empty() {
        return;
    }
    if !state.filtered_indices.contains(&state.selected) {
        state.selected = state.filtered_indices[0];
    }
}

/// Copy text to the system clipboard, trying multiple platform-specific tools.
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::CommitDetail;
    use chrono::Utc;
    use std::collections::HashMap;

    fn week(name: &str, authors: &[&str]) -> WeekStats {
        WeekStats {
            week: name.to_string(),
            commits: 1,
            lines_added: 1,
            lines_deleted: 0,
            top_authors: authors.iter().map(|a| a.to_string()).collect(),
            file_extensions: HashMap::new(),
            top_files: Vec::new(),
        }
    }

    #[test]
    fn search_filter_limits_indices_and_selection() {
        let weeks = vec![
            week("2024-W01", &["alice"]),
            week("2024-W02", &["bob"]),
            week("2024-W03", &["carol"]),
        ];
        let mut state = TuiState::default();
        state.selected = 2;
        state.search_query = "w02".into();

        apply_search_filter(&weeks, &mut state);

        assert_eq!(state.filtered_indices, vec![1]);
        assert_eq!(state.selected, 1, "selection should move into filtered set");

        state.search_query = "carol".into();
        apply_search_filter(&weeks, &mut state);
        assert_eq!(state.filtered_indices, vec![2]);
        assert_eq!(state.selected, 2, "author match should be respected");
    }

    fn commit_detail(short_hash: &str, author: &str, message: &str) -> CommitDetail {
        CommitDetail {
            hash: format!("{short_hash}0000"),
            short_hash: short_hash.to_string(),
            message: message.to_string(),
            author_name: author.to_string(),
            author_email: format!("{author}@example.com"),
            timestamp: Utc::now(),
            files_changed: vec![],
            lines_added: 1,
            lines_deleted: 0,
        }
    }

    #[test]
    fn commit_search_filters_and_trims_selection() {
        let mut state = TuiState::default();
        state.commit_details = vec![
            commit_detail("a1", "Alice", "initial commit"),
            commit_detail("b2", "Bob", "feature work"),
        ];
        state.commit_selected = 5;
        state.commit_search_query = "bob".into();

        apply_commit_search_filter(&mut state);

        assert_eq!(state.commit_filtered_indices, vec![1]);
        assert_eq!(
            state.commit_selected, 0,
            "selection should clamp when filtered list shrinks"
        );

        state.commit_search_query = "feature".into();
        apply_commit_search_filter(&mut state);
        assert_eq!(state.commit_filtered_indices, vec![1]);
    }
}
