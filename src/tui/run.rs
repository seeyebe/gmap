use std::io;
use crossterm::terminal::{enable_raw_mode, disable_raw_mode};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use super::state::{TuiState, WeekStats, ViewMode};
use super::input::{apply_search_filter, ensure_selection_in_filtered};
use crossterm::event::{poll, read, Event, KeyCode};
use crossterm::event::{KeyEventKind};
use super::views::{
    draw_heatmap_view,
    draw_statistics_view,
    draw_filetypes_view,
    draw_timeline_view,
    draw_help_overlay,
};

pub fn run(weeks: Vec<WeekStats>) -> io::Result<()> {
    enable_raw_mode()?;
    let mut terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;
    let mut state = TuiState::default();

    state.filtered_indices = (0..weeks.len()).collect();
    terminal.clear()?;

    loop {
        let draw_result = terminal.draw(|f| {
            let size = f.size();

            if state.show_help {
                draw_help_overlay(f, size);
                return;
            }

            let chunks = ratatui::layout::Layout::default()
                .direction(ratatui::layout::Direction::Vertical)
                .constraints([
                    ratatui::layout::Constraint::Length(3),
                    ratatui::layout::Constraint::Min(0),
                ])
                .split(size);

            let tabs = ratatui::widgets::Tabs::new(vec!["Heatmap", "Stats", "Files", "Timeline"])
                .block(ratatui::widgets::Block::default().borders(ratatui::widgets::Borders::ALL).title("View Mode"))
                .highlight_style(ratatui::style::Style::default().fg(ratatui::style::Color::Yellow).add_modifier(ratatui::style::Modifier::BOLD))
                .select(state.tab_index);
            f.render_widget(tabs, chunks[0]);

            state.view_mode = match state.tab_index {
                0 => ViewMode::Heatmap,
                1 => ViewMode::Statistics,
                2 => ViewMode::FileTypes,
                3 => ViewMode::Timeline,
                _ => ViewMode::Heatmap,
            };

            match state.view_mode {
                ViewMode::Heatmap => draw_heatmap_view(f, chunks[1], &weeks, &state),
                ViewMode::Statistics => draw_statistics_view(f, chunks[1], &weeks, &state),
                ViewMode::FileTypes => draw_filetypes_view(f, chunks[1], &weeks, &state),
                ViewMode::Timeline => draw_timeline_view(f, chunks[1], &weeks, &state),
            }
        });

        if let Err(e) = draw_result {
            eprintln!("TUI draw error: {}", e);
        }

        if poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(key_event) = read()? {
                if key_event.kind != KeyEventKind::Press {
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
                        KeyCode::Tab => {
                            state.tab_index = (state.tab_index + 1) % 4;
                        }
                        KeyCode::BackTab => {
                            state.tab_index = if state.tab_index == 0 { 3 } else { state.tab_index - 1 };
                        }
                        KeyCode::Left | KeyCode::Char('j') => {
                            if state.selected > 0 {
                                state.selected -= 1;
                                ensure_selection_in_filtered(&mut state);
                            }
                        }
                        KeyCode::Right | KeyCode::Char('k') => {
                            if state.selected + 1 < weeks.len() {
                                state.selected += 1;
                                ensure_selection_in_filtered(&mut state);
                            }
                        }
                        KeyCode::Home => {
                            state.selected = 0;
                            ensure_selection_in_filtered(&mut state);
                        }
                        KeyCode::End => {
                            state.selected = weeks.len().saturating_sub(1);
                            ensure_selection_in_filtered(&mut state);
                        }
                        KeyCode::PageUp => {
                            state.selected = state.selected.saturating_sub(10);
                            ensure_selection_in_filtered(&mut state);
                        }
                        KeyCode::PageDown => {
                            state.selected = std::cmp::min(state.selected + 10, weeks.len().saturating_sub(1));
                            ensure_selection_in_filtered(&mut state);
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    terminal.clear()?;
    disable_raw_mode()?;
    Ok(())
}
