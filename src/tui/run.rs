use std::cell::RefCell;
use std::io;
use std::time::Duration;

use crossterm::event::{poll, read, DisableMouseCapture, EnableMouseCapture, Event};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    widgets::{Block, Borders, Tabs},
    Terminal,
};

use crate::cache::Cache;
use crate::cli::CommonArgs;
use crate::git::GitRepo;
use crate::heat::aggregate_weeks;

use super::events::{handle_key_events, handle_mouse_event};
use super::state::{TuiState, ViewMode};
use super::views::{
    draw_commit_details_view, draw_file_modal, draw_heatmap_view, draw_help_overlay,
    draw_statistics_view, draw_timeline_view,
};

/// Launch the interactive TUI, handling setup, draw loop, and event dispatch.
pub fn run(common: &CommonArgs, path: Option<String>, monthly: bool) -> io::Result<()> {
    let repo = GitRepo::open(common.repo.as_ref()).map_err(io::Error::other)?;
    let mut cache = Cache::new(common.cache.as_deref(), repo.path()).map_err(io::Error::other)?;
    let range = repo
        .resolve_range(common.since.as_deref(), common.until.as_deref())
        .map_err(io::Error::other)?;
    let mut include_merges_state = common.include_merges;
    let mut monthly_state = monthly;

    let mut stats = crate::heat::fetch_commit_stats_with_progress(
        &repo,
        &mut cache,
        &range,
        include_merges_state,
        common.binary,
        false,
    )
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
        if weeks.len() > limit {
            weeks = weeks.split_off(weeks.len() - limit);
        }
    }
    state.filtered_indices = (0..weeks.len()).collect();
    terminal.clear()?;

    loop {
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
                render_tabs(f, &state, chunks[0]);
                match state.view_mode {
                    ViewMode::Heatmap => draw_heatmap_view(f, chunks[1], &weeks, &state),
                    ViewMode::Statistics => draw_statistics_view(f, chunks[1], &weeks, &state),
                    ViewMode::Timeline => draw_timeline_view(f, chunks[1], &weeks, &state),
                    ViewMode::CommitDetails => {
                        draw_commit_details_view(f, chunks[1], &weeks, &mut state)
                    }
                }
                draw_file_modal(f, size, &weeks[state.selected]);
                return;
            }

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),
                    Constraint::Min(0),
                    Constraint::Length(1),
                ])
                .split(size);

            render_tabs(f, &state, chunks[0]);

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
                ViewMode::CommitDetails => {
                    draw_commit_details_view(f, chunks[1], &weeks, &mut state)
                }
            }

            draw_prompt(f, &state, chunks[2]);
        }) {
            eprintln!("TUI draw error: {e}");
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
                    let quit = handle_key_events(
                        key_event,
                        &mut state,
                        &mut weeks,
                        &mut stats,
                        &mut cache,
                        path.as_deref(),
                        common,
                        &repo,
                        &range,
                        &gi,
                        &mut monthly_state,
                        &mut include_merges_state,
                    )?;
                    if quit {
                        break;
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

/// Render the view-mode tabs for the active layout.
fn render_tabs(f: &mut ratatui::Frame, state: &TuiState, area: Rect) {
    let titles = ["Heatmap", "Stats", "Timeline", "Commits"];
    let tab_items: Vec<String> = titles.iter().map(|t| t.to_string()).collect();
    let tabs = Tabs::new(tab_items)
        .block(Block::default().borders(Borders::ALL).title("View Mode"))
        .select(state.tab_index);
    f.render_widget(tabs, area);
}

/// Draw the bottom prompt/status line depending on active input modes.
fn draw_prompt(f: &mut ratatui::Frame, state: &TuiState, area: Rect) {
    use ratatui::widgets::Paragraph;
    if state.search_mode {
        let p = Paragraph::new(format!(
            "Period filter: {} (Enter to apply, Esc to cancel)",
            state.search_query
        ));
        f.render_widget(p, area);
    } else if state.commit_search_mode {
        let p = Paragraph::new(format!(
            "Commit filter: {} (Enter to apply, Esc to cancel)",
            state.commit_search_query
        ));
        f.render_widget(p, area);
    } else if state.path_mode {
        let p = Paragraph::new(format!(
            "Path prefix: {} (Enter to apply, Esc to cancel)",
            state.path_input
        ));
        f.render_widget(p, area);
    } else if let Some((message, ts)) = &state.status_message {
        if ts.elapsed().as_millis() < 2500 {
            let p = Paragraph::new(message.clone());
            f.render_widget(p, area);
        }
    }
}
