use ratatui::style::{Color, Modifier, Style};

const LEVELS: [&str; 8] = ["▁", "▂", "▃", "▄", "▅", "▆", "▇", "█"];

/// Returns a compact intensity bar of fixed width (3) based on commits/max.
pub fn enhanced_intensity_bar(commits: usize, max: usize) -> String {
    const WIDTH: usize = 3;
    if max == 0 {
        return "▁▁▁".to_string();
    }

    let ratio = commits as f64 / max as f64;
    let filled = ((ratio * WIDTH as f64).round() as usize).min(WIDTH);
    let intensity_idx = ((ratio * (LEVELS.len() - 1) as f64).round() as usize)
        .min(LEVELS.len() - 1);

    let bar_char = LEVELS[intensity_idx];
    bar_char.repeat(filled) + &"░".repeat(WIDTH - filled)
}

/// Chooses a style/color based on relative intensity of commit activity.
pub fn get_intensity_color(commits: usize, max: usize) -> Style {
    if max == 0 {
        return Style::default().fg(Color::White);
    }

    let ratio = commits as f64 / max as f64;
    if ratio > 0.8 {
        Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
    } else if ratio > 0.6 {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else if ratio > 0.4 {
        Style::default().fg(Color::Green)
    } else if ratio > 0.2 {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::Blue)
    }
}
