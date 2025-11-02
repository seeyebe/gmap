use std::io;
use std::time::Duration;
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crossterm::event::{
    poll, read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind,
    MouseButton, MouseEvent, MouseEventKind,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    widgets::{Tabs, Block, Borders},
    Terminal,
};

use super::state::{TuiState, ViewMode};
use super::input::{apply_search_filter, apply_commit_search_filter, ensure_selection_in_filtered, copy_to_clipboard};
use super::views::{
    draw_commit_details_view, draw_help_overlay, draw_heatmap_view,
    draw_statistics_view, draw_timeline_view, draw_file_modal,
};

use crate::cli::CommonArgs;
use crate::git::GitRepo;
use crate::cache::Cache;
use crate::heat::{aggregate_weeks, load_commit_details};
use std::cell::RefCell;

pub fn run(common: &CommonArgs, path: Option<String>, monthly: bool) -> io::Result<()> {
    let repo = GitRepo::open(common.repo.as_ref()).map_err(|e| io::Error::other(e))?;
    let mut cache = Cache::new(common.cache.as_deref(), repo.path())
        .map_err(|e| io::Error::other(e))?;
    let range = repo
        .resolve_range(common.since.as_deref(), common.until.as_deref())
        .map_err(|e| io::Error::other(e))?;
    let mut include_merges_state = common.include_merges;
    let mut monthly_state = monthly;

    let mut stats = crate::heat::fetch_commit_stats_with_progress(&repo, &mut cache, &range, include_merges_state, common.binary, false)
        .map_err(io::Error::other)?;
    let gi = RefCell::new(crate::util::GitIgnoreMatcher::new(repo.path()));
    let mut weeks = aggregate_weeks(
        &stats,
        &cache,
        path.as_deref(),
        common.author.as_deref(),
        common.author_email.as_deref(),
        monthly_state,
        &common.exclude,
        Some(&gi),
    );

    enable_raw_mode()?;
    crossterm::execute!(io::stdout(), EnableMouseCapture)?;

    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut state = TuiState::default();
    if !state.show_all {
        let limit = if monthly_state { 12 } else { 52 };
        if weeks.len() > limit { weeks = weeks.split_off(weeks.len() - limit); }
    }
    state.filtered_indices = (0..weeks.len()).collect();
    terminal.clear()?;

    loop {
        // Expire transient status messages
        if let Some((_, t)) = &state.status_message {
            if t.elapsed().as_secs() >= 3 {
                state.status_message = None;
            }
        }

        if let Err(e) = terminal.draw(|f| {
            let size = f.size();

            if state.show_help {
                draw_help_overlay(f, size);
                return;
            }

            if state.show_file_modal {
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Length(3), Constraint::Min(0)])
                    .split(size);
                let titles = ["Heatmap", "Stats", "Timeline", "Commits"];
                let tab_items: Vec<String> = titles.iter().map(|t| t.to_string()).collect();
                let tabs = Tabs::new(tab_items)
                    .block(Block::default().borders(Borders::ALL).title("View Mode"))
                    .select(state.tab_index);
                f.render_widget(tabs, chunks[0]);

                match state.view_mode {
                    ViewMode::Heatmap => draw_heatmap_view(f, chunks[1], &weeks, &state),
                    ViewMode::Statistics => draw_statistics_view(f, chunks[1], &weeks, &state),
                    ViewMode::Timeline => draw_timeline_view(f, chunks[1], &weeks, &state),
                    ViewMode::CommitDetails => draw_commit_details_view(f, chunks[1], &weeks, &mut state),
                }

                draw_file_modal(f, size, &weeks[state.selected]);
                return;
            }

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(3), Constraint::Min(0), Constraint::Length(1)])
                .split(size);

            let titles = ["Heatmap", "Stats", "Timeline", "Commits"];
            let tab_items: Vec<String> = titles.iter().map(|t| t.to_string()).collect();
            let tabs = Tabs::new(tab_items)
                .block(Block::default().borders(Borders::ALL).title("View Mode"))
                .select(state.tab_index);
            f.render_widget(tabs, chunks[0]);

            state.view_mode = match state.tab_index {
                0 => ViewMode::Heatmap,
                1 => ViewMode::Statistics,
                2 => ViewMode::Timeline,
                3 => ViewMode::CommitDetails,
                _ => ViewMode::Heatmap,
            };

            match state.view_mode {
                ViewMode::Heatmap => draw_heatmap_view(f, chunks[1], &weeks, &state),
                ViewMode::Statistics => draw_statistics_view(f, chunks[1], &weeks, &state),
                ViewMode::Timeline => draw_timeline_view(f, chunks[1], &weeks, &state),
                ViewMode::CommitDetails => draw_commit_details_view(f, chunks[1], &weeks, &mut state),
            }

            // Prompt / status line
            use ratatui::widgets::Paragraph;
            if state.search_mode {
                let p = Paragraph::new(format!("Period filter: {} (Enter to apply, Esc to cancel)", state.search_query));
                f.render_widget(p, chunks[2]);
            } else if state.commit_search_mode {
                let p = Paragraph::new(format!("Commit filter: {} (Enter to apply, Esc to cancel)", state.commit_search_query));
                f.render_widget(p, chunks[2]);
            } else if state.path_mode {
                let p = Paragraph::new(format!("Path prefix: {} (Enter to apply, Esc to cancel)", state.path_input));
                f.render_widget(p, chunks[2]);
            } else if let Some((message, ts)) = &state.status_message {
                if ts.elapsed().as_millis() < 2500 {
                    let p = Paragraph::new(message.clone());
                    f.render_widget(p, chunks[2]);
                }
            }
        }) {
            eprintln!("TUI draw error: {}", e);
        }

        if poll(Duration::from_millis(200))? {
            match read()? {
                Event::Mouse(mouse_event) => {
                    handle_mouse_event(
                        mouse_event,
                        &mut state,
                        &weeks,
                        &stats,
                        &cache,
                        path.as_deref(),
                        monthly_state,
                    )?;
                }
                Event::Key(key_event) => {
                    if key_event.kind != KeyEventKind::Press {
                        continue;
                    }

                    if state.show_file_modal {
                        if let KeyCode::Esc = key_event.code {
                            state.show_file_modal = false;
                        }
                        continue;
                    }

                    if state.search_mode {
                        match key_event.code {
                            KeyCode::Esc => {
                                state.search_mode = false;
                                state.search_query.clear();
                                state.filtered_indices = (0..weeks.len()).collect();
                            }
                            KeyCode::Enter => { state.search_mode = false; apply_search_filter(&weeks, &mut state); }
                            KeyCode::Backspace => {
                                state.search_query.pop();
                                apply_search_filter(&weeks, &mut state);
                            }
                            KeyCode::Char(c) => {
                                state.search_query.push(c);
                                apply_search_filter(&weeks, &mut state);
                            }
                            _ => {}
                        }
                    } else if state.commit_search_mode {
                        match key_event.code {
                            KeyCode::Esc => {
                                state.commit_search_mode = false;
                                state.commit_search_query.clear();
                                state.commit_filtered_indices = (0..state.commit_details.len()).collect();
                            }
                            KeyCode::Enter => {
                                state.commit_search_mode = false;
                                apply_commit_search_filter(&mut state);
                            }
                            KeyCode::Backspace => {
                                state.commit_search_query.pop();
                                apply_commit_search_filter(&mut state);
                            }
                            KeyCode::Char(c) => {
                                state.commit_search_query.push(c);
                                apply_commit_search_filter(&mut state);
                            }
                            _ => {}
                        }
                    } else if state.path_mode {
                        match key_event.code {
                            KeyCode::Esc => {
                                state.path_mode = false;
                                state.path_input.clear();
                            }
                            KeyCode::Enter => {
                                state.path_mode = false;
                                let trimmed = state.path_input.trim();
                                state.path_filter = if trimmed.is_empty() { None } else { Some(trimmed.to_string()) };
                                weeks = aggregate_weeks(
                                    &stats,
                                    &cache,
                                    state.path_filter.as_deref().or(path.as_deref()),
                                    common.author.as_deref(),
                                    common.author_email.as_deref(),
                                    monthly_state,
                                    &common.exclude,
                                    Some(&gi),
                                );
                                if !state.show_all {
                                    let limit = if monthly_state { 12 } else { 52 };
                                    if weeks.len() > limit { weeks = weeks.split_off(weeks.len() - limit); }
                                }
                                state.filtered_indices = (0..weeks.len()).collect();
                                state.commit_details.clear();
                                state.commit_selected = 0;
                                state.commit_filtered_indices.clear();
                            }
                            KeyCode::Backspace => { state.path_input.pop(); }
                            KeyCode::Char(c) => { state.path_input.push(c); }
                            _ => {}
                        }
                    } else {
                        match key_event.code {
                            KeyCode::Char('q') => break,
                            KeyCode::Char('h') | KeyCode::F(1) => state.show_help = !state.show_help,
                            KeyCode::Char('/') => {
                                state.search_mode = true;
                                state.search_query.clear();
                            }
                            KeyCode::Char(':') => { state.commit_search_mode = true; state.commit_search_query.clear(); }
                            KeyCode::Enter => {
                                if state.view_mode != ViewMode::CommitDetails && !weeks.is_empty() && state.selected < weeks.len() {
                                    let active_path_owned = state.path_filter.clone().or_else(|| path.clone());
                                    let active_path = active_path_owned.as_deref();
                                    if let Err(e) = crate::heat::load_commit_details(&mut state, &weeks, &stats, &cache, active_path, common.author.as_deref(), common.author_email.as_deref(), monthly_state) {
                                        state.status_message = Some((format!("Load error: {}", e), std::time::Instant::now()));
                                    } else {
                                        state.commit_filtered_indices = (0..state.commit_details.len()).collect();
                                        state.view_mode = ViewMode::CommitDetails;
                                        state.tab_index = 3;
                                    }
                                }
                            }
                            KeyCode::Char('p') => { state.path_mode = true; state.path_input = state.path_filter.clone().unwrap_or_default(); }
                            KeyCode::Char('c') => {
                                if let Some(commit) = state.commit_details.get(state.commit_selected) {
                                    match copy_to_clipboard(&commit.hash) {
                                        Ok(_) => {
                                            state.status_message = Some((
                                                format!("Copied: {}", commit.short_hash),
                                                std::time::Instant::now(),
                                            ));
                                        }
                                        Err(err) => {
                                            state.status_message = Some((
                                                format!("Clipboard error: {}", err),
                                                std::time::Instant::now(),
                                            ));
                                        }
                                    }
                                }
                            }
                            KeyCode::Char('y') => {
                                if let Some(commit) = state.commit_details.get(state.commit_selected) {
                                    let _ = copy_to_clipboard(&commit.short_hash);
                                    state.status_message = Some((
                                        format!("Copied: {}", commit.short_hash),
                                        std::time::Instant::now(),
                                    ));
                                }
                            }
                            KeyCode::Char('o') => {
                                if let Some(commit) = state.commit_details.get(state.commit_selected) {
                                    let _ = disable_raw_mode();
                                    let _ = std::process::Command::new("sh")
                                        .arg("-c")
                                        .arg(format!(
                                            "git -C '{}' show --stat {} | ${{PAGER:-less -R}}",
                                            repo.path().display(),
                                            commit.hash
                                        ))
                                        .status();
                                    let _ = enable_raw_mode();
                                }
                            }
                            KeyCode::Char('m') => {
                                // throttle
                                let now = std::time::Instant::now();
                                if let Some(t) = state.last_refresh { if now.duration_since(t).as_millis() < 300 { break; } }
                                state.last_refresh = Some(now);
                                // Toggle monthly/weekly aggregation
                                monthly_state = !monthly_state;
                                weeks = aggregate_weeks(
                                    &stats,
                                    &cache,
                                    state.path_filter.as_deref().or(path.as_deref()),
                                    common.author.as_deref(),
                                    common.author_email.as_deref(),
                                    monthly_state,
                                    &common.exclude,
                                    Some(&gi),
                                );
                                if !state.show_all {
                                    let limit = if monthly_state { 12 } else { 52 };
                                    if weeks.len() > limit { weeks = weeks.split_off(weeks.len() - limit); }
                                }
                                apply_search_filter(&weeks, &mut state);
                                if !weeks.is_empty() {
                                    let active_path_owned = state.path_filter.clone().or_else(|| path.clone());
                                    let active_path = active_path_owned.as_deref();
                                    let _ = crate::heat::load_commit_details(&mut state, &weeks, &stats, &cache, active_path, common.author.as_deref(), common.author_email.as_deref(), monthly_state);
                                    state.commit_filtered_indices = (0..state.commit_details.len()).collect();
                                }
                            }
                            KeyCode::Char('M') => {
                                // throttle
                                let now = std::time::Instant::now();
                                if let Some(t) = state.last_refresh { if now.duration_since(t).as_millis() < 300 { break; } }
                                state.last_refresh = Some(now);
                                // Toggle include merges and refetch stats
                                include_merges_state = !include_merges_state;
                                stats = crate::heat::fetch_commit_stats_with_progress(&repo, &mut cache, &range, include_merges_state, common.binary, false)
                                .map_err(|e| io::Error::other(e))?;
                                weeks = aggregate_weeks(
                                    &stats,
                                    &cache,
                                    state.path_filter.as_deref().or(path.as_deref()),
                                    common.author.as_deref(),
                                    common.author_email.as_deref(),
                                    monthly_state,
                                    &common.exclude,
                                    Some(&gi),
                                );
                                if !state.show_all {
                                    let limit = if monthly_state { 12 } else { 52 };
                                    if weeks.len() > limit { weeks = weeks.split_off(weeks.len() - limit); }
                                }
                                apply_search_filter(&weeks, &mut state);
                            }
                            KeyCode::Char('A') => {
                                state.show_all = !state.show_all;
                                if state.show_all {
                                    weeks = aggregate_weeks(
                                        &stats,
                                        &cache,
                                        state.path_filter.as_deref().or(path.as_deref()),
                                        common.author.as_deref(),
                                        common.author_email.as_deref(),
                                        monthly_state,
                                        &common.exclude,
                                        Some(&gi),
                                    );
                                } else {
                                    let limit = if monthly_state { 12 } else { 52 };
                                    if weeks.len() > limit { weeks = weeks.split_off(weeks.len() - limit); }
                                }
                                apply_search_filter(&weeks, &mut state);
                                if !weeks.is_empty() {
                                    let active_path_owned = state.path_filter.clone().or_else(|| path.clone());
                                    let active_path = active_path_owned.as_deref();
                                    let _ = crate::heat::load_commit_details(&mut state, &weeks, &stats, &cache, active_path, common.author.as_deref(), common.author_email.as_deref(), monthly_state);
                                    state.commit_filtered_indices = (0..state.commit_details.len()).collect();
                                }
                            }
                            KeyCode::Tab => { state.tab_index = (state.tab_index + 1) % 4; }
                            KeyCode::BackTab => { state.tab_index = if state.tab_index == 0 { 3 } else { state.tab_index - 1 }; }
                            KeyCode::Up | KeyCode::Char('k') => {
                                if state.view_mode == ViewMode::CommitDetails {
                                    if !state.commit_details.is_empty() {
                                        if !state.commit_search_query.is_empty() && !state.commit_filtered_indices.is_empty() {
                                            if let Some(pos) = state.commit_filtered_indices.iter().position(|&i| i == state.commit_selected) {
                                                if pos > 0 { state.commit_selected = state.commit_filtered_indices[pos - 1]; }
                                            }
                                        } else {
                                            state.commit_selected = state.commit_selected.saturating_sub(1);
                                        }
                                    }
                                } else if state.selected > 0 {
                                    state.selected -= 1;
                                    ensure_selection_in_filtered(&mut state);
                                }
                            }
                            KeyCode::Down | KeyCode::Char('j') => {
                                if state.view_mode == ViewMode::CommitDetails {
                                    if !state.commit_details.is_empty() {
                                        if !state.commit_search_query.is_empty() && !state.commit_filtered_indices.is_empty() {
                                            if let Some(pos) = state.commit_filtered_indices.iter().position(|&i| i == state.commit_selected) {
                                                if pos + 1 < state.commit_filtered_indices.len() { state.commit_selected = state.commit_filtered_indices[pos + 1]; }
                                            }
                                        } else if state.commit_selected + 1 < state.commit_details.len() {
                                            state.commit_selected += 1;
                                        }
                                    }
                                } else if state.selected + 1 < weeks.len() {
                                    state.selected += 1;
                                    ensure_selection_in_filtered(&mut state);
                                }
                            }
                            KeyCode::Char('g') => {
                                if state.view_mode == ViewMode::CommitDetails { state.commit_selected = 0; } else { state.selected = 0; ensure_selection_in_filtered(&mut state); }
                            }
                            KeyCode::Char('G') => {
                                if state.view_mode == ViewMode::CommitDetails { state.commit_selected = state.commit_details.len().saturating_sub(1); } else { state.selected = weeks.len().saturating_sub(1); ensure_selection_in_filtered(&mut state); }
                            }
                            KeyCode::Home => {
                                if state.view_mode == ViewMode::CommitDetails {
                                    if !state.commit_search_query.is_empty() && !state.commit_filtered_indices.is_empty() {
                                        state.commit_selected = state.commit_filtered_indices[0];
                                    } else { state.commit_selected = 0; }
                                } else { state.selected = 0; }
                            }
                            KeyCode::End => {
                                if state.view_mode == ViewMode::CommitDetails {
                                    if !state.commit_search_query.is_empty() && !state.commit_filtered_indices.is_empty() {
                                        state.commit_selected = *state.commit_filtered_indices.last().unwrap();
                                    } else { state.commit_selected = state.commit_details.len().saturating_sub(1); }
                                } else { state.selected = weeks.len().saturating_sub(1); }
                            }
                            KeyCode::PageUp => { state.selected = state.selected.saturating_sub(10); state.commit_selected = state.commit_selected.saturating_sub(10); }
                            KeyCode::PageDown => { state.selected = std::cmp::min(state.selected + 10, weeks.len().saturating_sub(1)); state.commit_selected = std::cmp::min(state.commit_selected + 10, state.commit_details.len().saturating_sub(1)); }
                            _ => {}
                        }
                    }
                }
                _ => {}
            }
        }
    }

    crossterm::execute!(io::stdout(), DisableMouseCapture)?;
    terminal.clear()?;
    disable_raw_mode()?;
    Ok(())
}

fn handle_mouse_event(
    mouse_event: MouseEvent,
    state: &mut TuiState,
    weeks: &[super::state::WeekStats],
    stats: &[crate::model::CommitStats],
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
            if state.view_mode != ViewMode::CommitDetails && !weeks.is_empty() && state.selected < weeks.len() {
                if let Err(e) = load_commit_details(
                    state, weeks, stats, cache, path_prefix,
                    None, None, monthly,
                ) {
                    eprintln!("Error loading commit details: {}", e);
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
