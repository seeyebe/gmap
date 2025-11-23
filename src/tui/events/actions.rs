use std::cell::RefCell;
use std::io;

use crate::cache::Cache;
use crate::cli::CommonArgs;
use crate::git::GitRepo;
use crate::heat::{aggregate_weeks, load_commit_details};
use crate::model::{CommitStats, DateRange};
use crate::util::GitIgnoreMatcher;

use super::super::input::{apply_search_filter, copy_to_clipboard, ensure_selection_in_filtered};
use super::super::state::{TuiState, ViewMode, WeekStats};

/// Load commit details for the currently selected period and switch into the details view.
pub(super) fn try_load_commit_details(
    state: &mut TuiState,
    weeks: &[WeekStats],
    stats: &[CommitStats],
    cache: &Cache,
    path: Option<&str>,
    common: &CommonArgs,
    monthly_state: bool,
) {
    if state.view_mode == ViewMode::CommitDetails
        || weeks.is_empty()
        || state.selected >= weeks.len()
    {
        return;
    }

    let active_path_owned = state
        .path_filter
        .clone()
        .or_else(|| path.map(|p| p.to_string()));
    let active_path = active_path_owned.as_deref();
    match load_commit_details(
        state,
        weeks,
        stats,
        cache,
        active_path,
        common.author.as_deref(),
        common.author_email.as_deref(),
        monthly_state,
    ) {
        Ok(_) => {
            state.commit_filtered_indices = (0..state.commit_details.len()).collect();
            state.view_mode = ViewMode::CommitDetails;
            state.tab_index = 3;
        }
        Err(e) => {
            state.status_message = Some((format!("Load error: {e}"), std::time::Instant::now()));
        }
    }
}

/// Copy the full commit hash of the selected commit, surfacing clipboard errors in status.
pub(super) fn copy_full_hash(state: &mut TuiState) {
    if let Some(commit) = state.commit_details.get(state.commit_selected) {
        match copy_to_clipboard(&commit.hash) {
            Ok(_) => {
                state.status_message = Some((
                    format!("Copied: {}", commit.short_hash),
                    std::time::Instant::now(),
                ));
            }
            Err(err) => {
                state.status_message =
                    Some((format!("Clipboard error: {err}"), std::time::Instant::now()));
            }
        }
    }
}

/// Copy the short hash of the selected commit and show a transient status message.
pub(super) fn copy_short_hash(state: &mut TuiState) {
    if let Some(commit) = state.commit_details.get(state.commit_selected) {
        let _ = copy_to_clipboard(&commit.short_hash);
        state.status_message = Some((
            format!("Copied: {}", commit.short_hash),
            std::time::Instant::now(),
        ));
    }
}

/// Open the selected commit in the user's pager by spawning `git show` temporarily outside raw mode.
pub(super) fn open_commit_in_pager(state: &mut TuiState, repo: &GitRepo) {
    if let Some(commit) = state.commit_details.get(state.commit_selected) {
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!(
                "git -C '{}' show --stat {} | ${{PAGER:-less -R}}",
                repo.path().display(),
                commit.hash
            ))
            .status();
        let _ = crossterm::terminal::enable_raw_mode();
    }
}

/// Toggle weekly/monthly aggregation, re-aggregate data, and refresh commit filters.
pub(super) fn toggle_monthly(
    state: &mut TuiState,
    weeks: &mut Vec<WeekStats>,
    stats: &mut Vec<CommitStats>,
    cache: &mut Cache,
    path: Option<&str>,
    common: &CommonArgs,
    gi: &RefCell<GitIgnoreMatcher>,
    monthly_state: &mut bool,
) -> io::Result<()> {
    if should_throttle_refresh(state) {
        return Ok(());
    }
    *monthly_state = !*monthly_state;
    *weeks = aggregate_weeks(
        stats,
        cache,
        state.path_filter.as_deref().or(path),
        common.author.as_deref(),
        common.author_email.as_deref(),
        *monthly_state,
        &common.exclude,
        Some(gi),
    );
    if !state.show_all {
        let limit = if *monthly_state { 12 } else { 52 };
        if weeks.len() > limit {
            *weeks = weeks.split_off(weeks.len() - limit);
        }
    }
    apply_search_filter(weeks, state);
    if !weeks.is_empty() {
        let active_path_owned = state
            .path_filter
            .clone()
            .or_else(|| path.map(|p| p.to_string()));
        let active_path = active_path_owned.as_deref();
        let _ = load_commit_details(
            state,
            weeks,
            stats,
            cache,
            active_path,
            common.author.as_deref(),
            common.author_email.as_deref(),
            *monthly_state,
        );
        state.commit_filtered_indices = (0..state.commit_details.len()).collect();
    }
    Ok(())
}

/// Toggle inclusion of merge commits, refetch stats, and rebuild the current aggregation.
pub(super) fn toggle_merges(
    state: &mut TuiState,
    weeks: &mut Vec<WeekStats>,
    stats: &mut Vec<CommitStats>,
    cache: &mut Cache,
    path: Option<&str>,
    common: &CommonArgs,
    repo: &GitRepo,
    range: &DateRange,
    gi: &RefCell<GitIgnoreMatcher>,
    include_merges_state: &mut bool,
    monthly_state: bool,
) -> io::Result<()> {
    if should_throttle_refresh(state) {
        return Ok(());
    }
    *include_merges_state = !*include_merges_state;
    *stats = crate::heat::fetch_commit_stats_with_progress(
        repo,
        cache,
        range,
        *include_merges_state,
        common.binary,
        false,
    )
    .map_err(io::Error::other)?;
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
    apply_search_filter(weeks, state);
    Ok(())
}

/// Toggle between showing all periods or the recent subset and refresh derived state.
pub(super) fn toggle_show_all(
    state: &mut TuiState,
    weeks: &mut Vec<WeekStats>,
    stats: &mut Vec<CommitStats>,
    cache: &mut Cache,
    path: Option<&str>,
    common: &CommonArgs,
    gi: &RefCell<GitIgnoreMatcher>,
    monthly_state: bool,
) -> io::Result<()> {
    state.show_all = !state.show_all;
    if state.show_all {
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
    } else {
        let limit = if monthly_state { 12 } else { 52 };
        if weeks.len() > limit {
            *weeks = weeks.split_off(weeks.len() - limit);
        }
    }
    apply_search_filter(weeks, state);
    if !weeks.is_empty() {
        let active_path_owned = state
            .path_filter
            .clone()
            .or_else(|| path.map(|p| p.to_string()));
        let active_path = active_path_owned.as_deref();
        let _ = load_commit_details(
            state,
            weeks,
            stats,
            cache,
            active_path,
            common.author.as_deref(),
            common.author_email.as_deref(),
            monthly_state,
        );
        state.commit_filtered_indices = (0..state.commit_details.len()).collect();
    }
    Ok(())
}

/// Move selection upward respecting the current view and filtered commit indices.
pub(super) fn move_up(state: &mut TuiState) {
    if state.view_mode == ViewMode::CommitDetails {
        if !state.commit_details.is_empty() {
            if !state.commit_search_query.is_empty() && !state.commit_filtered_indices.is_empty() {
                if let Some(pos) = state
                    .commit_filtered_indices
                    .iter()
                    .position(|&i| i == state.commit_selected)
                {
                    if pos > 0 {
                        state.commit_selected = state.commit_filtered_indices[pos - 1];
                    }
                }
            } else {
                state.commit_selected = state.commit_selected.saturating_sub(1);
            }
        }
    } else if state.selected > 0 {
        state.selected -= 1;
        ensure_selection_in_filtered(state);
    }
}

/// Move selection downward respecting filtered commit indices and list bounds.
pub(super) fn move_down(state: &mut TuiState, weeks_len: usize) {
    if state.view_mode == ViewMode::CommitDetails {
        if !state.commit_details.is_empty() {
            if !state.commit_search_query.is_empty() && !state.commit_filtered_indices.is_empty() {
                if let Some(pos) = state
                    .commit_filtered_indices
                    .iter()
                    .position(|&i| i == state.commit_selected)
                {
                    if pos + 1 < state.commit_filtered_indices.len() {
                        state.commit_selected = state.commit_filtered_indices[pos + 1];
                    }
                }
            } else if state.commit_selected + 1 < state.commit_details.len() {
                state.commit_selected += 1;
            }
        }
    } else if state.selected + 1 < weeks_len {
        state.selected += 1;
        ensure_selection_in_filtered(state);
    }
}

/// Jump to the first item in the current list (periods or commits).
pub(super) fn jump_first(state: &mut TuiState) {
    if state.view_mode == ViewMode::CommitDetails {
        state.commit_selected = 0;
    } else {
        state.selected = 0;
        ensure_selection_in_filtered(state);
    }
}

/// Jump to the last item in the current list (periods or commits).
pub(super) fn jump_last(state: &mut TuiState, weeks_len: usize) {
    if state.view_mode == ViewMode::CommitDetails {
        state.commit_selected = state.commit_details.len().saturating_sub(1);
    } else {
        state.selected = weeks_len.saturating_sub(1);
        ensure_selection_in_filtered(state);
    }
}

/// Jump to the first filtered item in the active commit list or the first period.
pub(super) fn jump_home(state: &mut TuiState) {
    if state.view_mode == ViewMode::CommitDetails {
        if !state.commit_search_query.is_empty() && !state.commit_filtered_indices.is_empty() {
            state.commit_selected = state.commit_filtered_indices[0];
        } else {
            state.commit_selected = 0;
        }
    } else {
        state.selected = 0;
    }
}

/// Jump to the last filtered item in the active commit list or the last period.
pub(super) fn jump_end(state: &mut TuiState, weeks_len: usize) {
    if state.view_mode == ViewMode::CommitDetails {
        if !state.commit_search_query.is_empty() && !state.commit_filtered_indices.is_empty() {
            state.commit_selected = *state.commit_filtered_indices.last().unwrap();
        } else {
            state.commit_selected = state.commit_details.len().saturating_sub(1);
        }
    } else {
        state.selected = weeks_len.saturating_sub(1);
    }
}

/// Throttle rapid refresh actions to avoid expensive re-computation.
fn should_throttle_refresh(state: &mut TuiState) -> bool {
    let now = std::time::Instant::now();
    if let Some(t) = state.last_refresh {
        if now.duration_since(t).as_millis() < 300 {
            return true;
        }
    }
    state.last_refresh = Some(now);
    false
}
