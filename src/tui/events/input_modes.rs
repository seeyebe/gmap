use std::cell::RefCell;
use std::io;

use crossterm::event::KeyCode;

use crate::cache::Cache;
use crate::cli::CommonArgs;
use crate::heat::aggregate_weeks;
use crate::model::CommitStats;
use crate::util::GitIgnoreMatcher;

use super::super::input::{apply_commit_search_filter, apply_search_filter};
use super::super::state::{TuiState, WeekStats};

/// Handle period search keystrokes, applying filters on every change.
pub(super) fn handle_search_input(code: KeyCode, state: &mut TuiState, weeks: &[WeekStats]) {
    match code {
        KeyCode::Esc => {
            state.search_mode = false;
            state.search_query.clear();
            state.filtered_indices = (0..weeks.len()).collect();
        }
        KeyCode::Enter => {
            state.search_mode = false;
            apply_search_filter(weeks, state);
        }
        KeyCode::Backspace => {
            state.search_query.pop();
            apply_search_filter(weeks, state);
        }
        KeyCode::Char(c) => {
            state.search_query.push(c);
            apply_search_filter(weeks, state);
        }
        _ => {}
    }
}

/// Handle commit search keystrokes and re-apply commit filters.
pub(super) fn handle_commit_search_input(code: KeyCode, state: &mut TuiState) {
    match code {
        KeyCode::Esc => {
            state.commit_search_mode = false;
            state.commit_search_query.clear();
            state.commit_filtered_indices = (0..state.commit_details.len()).collect();
        }
        KeyCode::Enter => {
            state.commit_search_mode = false;
            apply_commit_search_filter(state);
        }
        KeyCode::Backspace => {
            state.commit_search_query.pop();
            apply_commit_search_filter(state);
        }
        KeyCode::Char(c) => {
            state.commit_search_query.push(c);
            apply_commit_search_filter(state);
        }
        _ => {}
    }
}

/// Handle path prefix input and re-aggregate data when the user submits a new path.
pub(super) fn handle_path_input(
    code: KeyCode,
    state: &mut TuiState,
    weeks: &mut Vec<WeekStats>,
    stats: &mut Vec<CommitStats>,
    cache: &mut Cache,
    path: Option<&str>,
    common: &CommonArgs,
    gi: &RefCell<GitIgnoreMatcher>,
    monthly_state: bool,
) -> io::Result<()> {
    match code {
        KeyCode::Esc => {
            state.path_mode = false;
            state.path_input.clear();
        }
        KeyCode::Enter => {
            state.path_mode = false;
            let trimmed = state.path_input.trim();
            state.path_filter = if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            };
            *weeks = aggregate_weeks(
                stats,
                cache,
                state.path_filter.as_deref().or(path),
                common.author.as_deref(),
                common.author_email.as_deref(),
                monthly_state,
                &common.exclude,
                Some(gi),
            );
            if !state.show_all {
                let limit = if monthly_state { 12 } else { 52 };
                if weeks.len() > limit {
                    *weeks = weeks.split_off(weeks.len() - limit);
                }
            }
            state.filtered_indices = (0..weeks.len()).collect();
            state.commit_details.clear();
            state.commit_selected = 0;
            state.commit_filtered_indices.clear();
        }
        KeyCode::Backspace => {
            state.path_input.pop();
        }
        KeyCode::Char(c) => {
            state.path_input.push(c);
        }
        _ => {}
    }
    Ok(())
}
