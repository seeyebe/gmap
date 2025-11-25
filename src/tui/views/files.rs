use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table};
use ratatui::Frame;

use crate::heat::FileExtensionStats;

use super::super::state::{TuiState, WeekStats};
use super::header_cell;

/// Render the file-type breakdown for the repository and the currently selected week.
pub fn draw_files_view(f: &mut Frame, area: Rect, weeks: &[WeekStats], state: &TuiState) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let mut overall: std::collections::HashMap<String, (usize, usize, usize, usize)> =
        std::collections::HashMap::new();
    for w in weeks {
        for (ext, s) in &w.file_extensions {
            let e = overall.entry(ext.clone()).or_insert((0, 0, 0, 0));
            e.0 += s.commits;
            e.1 += s.files_changed;
            e.2 += s.lines_added;
            e.3 += s.lines_deleted;
        }
    }

    let mut overall_vec: Vec<(String, usize, usize, usize, usize)> = overall
        .into_iter()
        .map(|(ext, v)| (ext, v.0, v.1, v.2, v.3))
        .collect();
    overall_vec.sort_by(|a, b| b.3.cmp(&a.3));

    let overall_rows: Vec<Row> = overall_vec
        .into_iter()
        .map(|(ext, commits, files, added, deleted)| {
            Row::new(vec![
                Cell::from(if ext.is_empty() {
                    "(none)".to_string()
                } else {
                    ext
                }),
                Cell::from(format!("{commits}")),
                Cell::from(format!("{files}")),
                Cell::from(format!("+{added}")).style(Style::default().fg(Color::Green)),
                Cell::from(format!("-{deleted}")).style(Style::default().fg(Color::Red)),
            ])
        })
        .collect();

    let overall_table = Table::new(
        overall_rows,
        [
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(12),
        ],
    )
    .header(Row::new([
        header_cell("Ext", Color::Yellow),
        header_cell("Commits", Color::Green),
        header_cell("Files", Color::Cyan),
        header_cell("Added", Color::Green),
        header_cell("Deleted", Color::Red),
    ]))
    .block(
        Block::default()
            .title("Overall File Types")
            .borders(Borders::ALL),
    );

    f.render_widget(overall_table, chunks[1]);

    if weeks.is_empty() || state.selected >= weeks.len() {
        let placeholder = Paragraph::new("No data").block(
            Block::default()
                .title("Selected Week")
                .borders(Borders::ALL),
        );
        f.render_widget(placeholder, chunks[0]);
        return;
    }

    let w = &weeks[state.selected];
    let mut week_vec: Vec<(&String, &FileExtensionStats)> = w.file_extensions.iter().collect();
    week_vec.sort_by(|a, b| b.1.lines_added.cmp(&a.1.lines_added));
    let week_rows: Vec<Row> = week_vec
        .into_iter()
        .map(|(ext, s)| {
            Row::new(vec![
                Cell::from(if ext.is_empty() {
                    "(none)".to_string()
                } else {
                    ext.clone()
                }),
                Cell::from(format!("{}", s.commits)),
                Cell::from(format!("{}", s.files_changed)),
                Cell::from(format!("+{}", s.lines_added)).style(Style::default().fg(Color::Green)),
                Cell::from(format!("-{}", s.lines_deleted)).style(Style::default().fg(Color::Red)),
            ])
        })
        .collect();

    let week_table = Table::new(
        week_rows,
        [
            Constraint::Length(12),
            Constraint::Length(10),
            Constraint::Length(12),
            Constraint::Length(12),
            Constraint::Length(12),
        ],
    )
    .header(Row::new([
        header_cell("Ext", Color::Yellow),
        header_cell("Commits", Color::Green),
        header_cell("Files", Color::Cyan),
        header_cell("Added", Color::Green),
        header_cell("Deleted", Color::Red),
    ]))
    .block(
        Block::default()
            .title(format!("File Types - {}", w.week))
            .borders(Borders::ALL),
    );

    f.render_widget(week_table, chunks[0]);
}
