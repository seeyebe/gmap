use ratatui::style::{Color, Modifier, Style};

pub fn enhanced_intensity_bar(commits: usize, max: usize) -> String {
    let levels = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];
    let width = 3;

    if max == 0 { return "▁▁▁".to_string(); }

    let ratio = commits as f64 / max as f64;
    let filled = ((ratio * width as f64).round() as usize).min(width);
    let intensity_idx = ((ratio * (levels.len() - 1) as f64).round() as usize).min(levels.len() - 1);

    let bar_char = levels[intensity_idx];
    bar_char.repeat(filled) + &"░".repeat(width - filled)
}

pub fn get_intensity_color(commits: usize, max: usize) -> Style {
    if max == 0 { return Style::default().fg(Color::White); }

    let ratio = commits as f64 / max as f64;
    match ratio {
        r if r > 0.8 => Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        r if r > 0.6 => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        r if r > 0.4 => Style::default().fg(Color::Green),
        r if r > 0.2 => Style::default().fg(Color::Cyan),
        _ => Style::default().fg(Color::Blue),
    }
}
