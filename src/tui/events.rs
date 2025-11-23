use std::cell::RefCell;
use std::io;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, MouseButton, MouseEvent, MouseEventKind};

use crate::cache::Cache;
use crate::cli::CommonArgs;
use crate::git::GitRepo;
use crate::heat::load_commit_details;
use crate::model::{CommitStats, DateRange};
use crate::util::GitIgnoreMatcher;

use super::input::ensure_selection_in_filtered;
use super::state::{TuiState, ViewMode, WeekStats};

mod actions;
mod input_modes;

use actions::*;
use input_modes::*;

/// Handle a keyboard event, mutating TUI state and returning `true` if the loop should exit.
pub fn handle_key_events(
    key_event: KeyEvent,
    state: &mut TuiState,
    weeks: &mut Vec<WeekStats>,
    stats: &mut Vec<CommitStats>,
    cache: &mut Cache,
    path: Option<&str>,
    common: &CommonArgs,
    repo: &GitRepo,
    range: &DateRange,
    gi: &RefCell<GitIgnoreMatcher>,
    monthly_state: &mut bool,
    include_merges_state: &mut bool,
) -> io::Result<bool> {
    if key_event.kind != KeyEventKind::Press {
        return Ok(false);
    }

    if state.show_file_modal {
        if let KeyCode::Esc = key_event.code {
            state.show_file_modal = false;
        }
        return Ok(false);
    }

    if state.search_mode {
        handle_search_input(key_event.code, state, weeks);
        return Ok(false);
    }

    if state.commit_search_mode {
        handle_commit_search_input(key_event.code, state);
        return Ok(false);
    }

    if state.path_mode {
        handle_path_input(
            key_event.code,
            state,
            weeks,
            stats,
            cache,
            path,
            common,
            gi,
            *monthly_state,
        )?;
        return Ok(false);
    }

    match key_event.code {
        KeyCode::Char('q') => return Ok(true),
        KeyCode::Char('h') | KeyCode::F(1) => state.show_help = !state.show_help,
        KeyCode::Char('/') => {
            state.search_mode = true;
            state.search_query.clear();
        }
        KeyCode::Char(':') => {
            state.commit_search_mode = true;
            state.commit_search_query.clear();
        }
        KeyCode::Enter => {
            try_load_commit_details(state, weeks, stats, cache, path, common, *monthly_state);
        }
        KeyCode::Char('p') => {
            state.path_mode = true;
            state.path_input = state.path_filter.clone().unwrap_or_default();
        }
        KeyCode::Char('c') => copy_full_hash(state),
        KeyCode::Char('y') => copy_short_hash(state),
        KeyCode::Char('o') => open_commit_in_pager(state, repo),
        KeyCode::Char('m') => {
            toggle_monthly(state, weeks, stats, cache, path, common, gi, monthly_state)?;
        }
        KeyCode::Char('M') => {
            toggle_merges(
                state,
                weeks,
                stats,
                cache,
                path,
                common,
                repo,
                range,
                gi,
                include_merges_state,
                *monthly_state,
            )?;
        }
        KeyCode::Char('A') => {
            toggle_show_all(state, weeks, stats, cache, path, common, gi, *monthly_state)?
        }
        KeyCode::Tab => state.tab_index = (state.tab_index + 1) % 4,
        KeyCode::BackTab => {
            state.tab_index = if state.tab_index == 0 {
                3
            } else {
                state.tab_index - 1
            };
        }
        KeyCode::Up | KeyCode::Char('k') => move_up(state),
        KeyCode::Down | KeyCode::Char('j') => move_down(state, weeks.len()),
        KeyCode::Char('g') => jump_first(state),
        KeyCode::Char('G') => jump_last(state, weeks.len()),
        KeyCode::Home => jump_home(state),
        KeyCode::End => jump_end(state, weeks.len()),
        KeyCode::PageUp => {
            state.selected = state.selected.saturating_sub(10);
            state.commit_selected = state.commit_selected.saturating_sub(10);
        }
        KeyCode::PageDown => {
            state.selected = std::cmp::min(state.selected + 10, weeks.len().saturating_sub(1));
            state.commit_selected = std::cmp::min(
                state.commit_selected + 10,
                state.commit_details.len().saturating_sub(1),
            );
        }
        _ => {}
    }

    Ok(false)
}

/// Handle mouse scrolling/click interactions for list navigation and commit loading.
pub fn handle_mouse_event(
    mouse_event: MouseEvent,
    state: &mut TuiState,
    weeks: &[WeekStats],
    stats: &[CommitStats],
    cache: &Cache,
    path_prefix: Option<&str>,
    monthly: bool,
) -> io::Result<()> {
    match mouse_event.kind {
        MouseEventKind::ScrollUp => {
            if state.view_mode == ViewMode::CommitDetails {
                state.commit_selected = state.commit_selected.saturating_sub(1);
            } else if state.selected > 0 {
                state.selected -= 1;
                ensure_selection_in_filtered(state);
            }
        }
        MouseEventKind::ScrollDown => {
            if state.view_mode == ViewMode::CommitDetails {
                if state.commit_selected + 1 < state.commit_details.len() {
                    state.commit_selected += 1;
                }
            } else if state.selected + 1 < weeks.len() {
                state.selected += 1;
                ensure_selection_in_filtered(state);
            }
        }
        MouseEventKind::Down(MouseButton::Left) => {
            if state.view_mode != ViewMode::CommitDetails
                && !weeks.is_empty()
                && state.selected < weeks.len()
            {
                if let Err(e) = load_commit_details(
                    state,
                    weeks,
                    stats,
                    cache,
                    path_prefix,
                    None,
                    None,
                    monthly,
                ) {
                    eprintln!("Error loading commit details: {e}");
                } else {
                    state.view_mode = ViewMode::CommitDetails;
                    state.tab_index = 3;
                }
            }
        }
        _ => {}
    }
    Ok(())
}
