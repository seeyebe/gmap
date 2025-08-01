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
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Tabs},
    Terminal,
};

use super::state::{TuiState, ViewMode};
use super::input::{apply_search_filter, ensure_selection_in_filtered, copy_to_clipboard};
use super::views::{
    draw_commit_details_view, draw_help_overlay, draw_heatmap_view,
    draw_statistics_view, draw_timeline_view, draw_file_modal,
};

use crate::cli::CommonArgs;
use crate::git::GitRepo;
use crate::cache::Cache;
use crate::heat::{aggregate_weeks, fetch_commit_stats, load_commit_details};

pub fn run(common: &CommonArgs, path: Option<String>) -> io::Result<()> {
    let repo = GitRepo::open(common.repo.as_ref()).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let mut cache = Cache::new(common.cache.as_deref(), repo.path())
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let range = repo
        .resolve_range(common.since.as_deref(), common.until.as_deref())
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let stats = fetch_commit_stats(&repo, &mut cache, &range, common.include_merges, common.binary)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let weeks = aggregate_weeks(&stats, &cache, path.as_deref());

    enable_raw_mode()?;
    crossterm::execute!(io::stdout(), EnableMouseCapture)?;

    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut state = TuiState::default();
    state.filtered_indices = (0..weeks.len()).collect();
    terminal.clear()?;

    loop {
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
                    .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
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
                .constraints([Constraint::Length(3), Constraint::Min(0)])
                .split(size);

            let titles = ["Heatmap", "Stats", "Timeline", "Commits"];
            let tab_items: Vec<String> = titles.iter().map(|t| t.to_string()).collect();
            let tabs = Tabs::new(tab_items)
                .block(Block::default().borders(Borders::ALL).title("View Mode"))
                .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
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
        }) {
            eprintln!("TUI draw error: {}", e);
        }

        if poll(Duration::from_millis(200))? {
            match read()? {
                Event::Mouse(mouse_event) => {
                    handle_mouse_event(mouse_event, &mut state, &weeks, &stats, &cache, path.as_deref())?;
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
                            KeyCode::Enter => {
                                state.search_mode = false;
                                apply_search_filter(&weeks, &mut state);
                            }
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
                    } else {
                        match key_event.code {
                            KeyCode::Char('q') => break,
                            KeyCode::Char('h') | KeyCode::F(1) => state.show_help = !state.show_help,
                            KeyCode::Char('/') => {
                                state.search_mode = true;
                                state.search_query.clear();
                            }
                            KeyCode::Char('f') => {
                                if !weeks.is_empty() && state.selected < weeks.len() {
                                    state.show_file_modal = !state.show_file_modal;
                                }
                            }
                            KeyCode::Enter => {
                                if state.view_mode != ViewMode::CommitDetails
                                    && !weeks.is_empty()
                                    && state.selected < weeks.len()
                                {
                                    load_commit_details(&mut state, &weeks, &stats, &cache, path.as_deref())?;
                                    state.view_mode = ViewMode::CommitDetails;
                                    state.tab_index = 3;
                                }
                            }
                            KeyCode::Char('c') => {
                                if state.view_mode == ViewMode::CommitDetails && !state.commit_details.is_empty() {
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
                            }
                            KeyCode::Tab => {
                                state.tab_index = (state.tab_index + 1) % 4;
                            }
                            KeyCode::BackTab => {
                                state.tab_index = if state.tab_index == 0 { 3 } else { state.tab_index - 1 };
                            }
                            KeyCode::Left | KeyCode::Char('j') => {
                                if state.view_mode == ViewMode::CommitDetails {
                                    state.commit_selected = state.commit_selected.saturating_sub(1);
                                } else if state.selected > 0 {
                                    state.selected -= 1;
                                    ensure_selection_in_filtered(&mut state);
                                }
                            }
                            KeyCode::Right | KeyCode::Char('k') => {
                                if state.view_mode == ViewMode::CommitDetails {
                                    if state.commit_selected + 1 < state.commit_details.len() {
                                        state.commit_selected += 1;
                                    }
                                } else if state.selected + 1 < weeks.len() {
                                    state.selected += 1;
                                    ensure_selection_in_filtered(&mut state);
                                }
                            }
                            KeyCode::Home => {
                                if state.view_mode == ViewMode::CommitDetails {
                                    state.commit_selected = 0;
                                } else {
                                    state.selected = 0;
                                    ensure_selection_in_filtered(&mut state);
                                }
                            }
                            KeyCode::End => {
                                if state.view_mode == ViewMode::CommitDetails {
                                    state.commit_selected = state.commit_details.len().saturating_sub(1);
                                } else {
                                    state.selected = weeks.len().saturating_sub(1);
                                    ensure_selection_in_filtered(&mut state);
                                }
                            }
                            KeyCode::PageUp => {
                                if state.view_mode == ViewMode::CommitDetails {
                                    state.commit_selected = state.commit_selected.saturating_sub(10);
                                } else {
                                    state.selected = state.selected.saturating_sub(10);
                                    ensure_selection_in_filtered(&mut state);
                                }
                            }
                            KeyCode::PageDown => {
                                if state.view_mode == ViewMode::CommitDetails {
                                    state.commit_selected = std::cmp::min(
                                        state.commit_selected + 10,
                                        state.commit_details.len().saturating_sub(1),
                                    );
                                } else {
                                    state.selected = std::cmp::min(state.selected + 10, weeks.len().saturating_sub(1));
                                    ensure_selection_in_filtered(&mut state);
                                }
                            }
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
                if let Err(e) = load_commit_details(state, weeks, stats, cache, path_prefix) {
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
