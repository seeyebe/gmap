# gmap

> A fast command-line tool to explore Git activity — heatmaps, churn, authorship, and more.

[![Crates.io](https://img.shields.io/crates/v/gmap.svg)](https://crates.io/crates/gmap)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

`gmap` helps you **understand your Git repository** at a glance — not just what changed, but *when, how much, and by whom*. Visualize commit activity over time, spot churn-heavy files, explore contributor dynamics, and more — all from your terminal.

Built for developers who live in the CLI and want quick, powerful insights.

---

## Features

- **Heatmap View**: Weekly commit activity with line additions, deletions, and churn
- **Churn Analysis**: File-level change volume over time
- **Timeline Sparklines**: See growth and change at a glance
- **Export Mode**: Output structured JSON for further analysis
- **Interactive TUI**: Terminal UI with search, filtering, and keyboard navigation

---

## Installation

Install via [crates.io](https://crates.io/crates/gmap):

```sh
cargo install gmap
````

---

## Getting Started

Run the interactive TUI on any Git repository:

```sh
gmap heat --tui
```

Use `Tab` or arrow keys to switch views. Press `h` for help.

---

## Example (TUI)

![TUI Heatmap](assets/tui-heatmap-preview.png)

---

## Why gmap?

When you’re dropped into a new codebase, or even trying to clean up your own, questions like these matter:

* Which files change the most?
* Who made most of the changes last month?
* Are there dormant areas of the code?
* What’s the trend of contributions over time?
* Where is most of the churn?

Traditional `git log` and `git blame` don’t answer these efficiently. `gmap` does.

---

## TUI Controls

| Key   | Action                 |
| ----- | ---------------------- |
| `← →` | Switch views           |
| `Tab` | Cycle through sections |
| `/`   | Search                 |
| `Esc` | Exit search            |
| `h`   | Toggle help overlay    |
| `q`   | Quit                   |

---

## Commands

```sh
gmap [OPTIONS] <COMMAND>
```

| Command  | Description                          |
| -------- | ------------------------------------ |
| `heat`   | Weekly commit heatmap (default view) |
| `churn`  | File-level change volume             |
| `export` | Export full stats as JSON            |
| `help`   | Show help message                    |

---

## Options

| Flag               | Purpose                             |
| ------------------ | ----------------------------------- |
| `--repo <path>`    | Git repo location (defaults to `.`) |
| `--cache <db>`     | Use or persist a cache DB           |
| `--since <date>`   | Analyze starting from date          |
| `--until <date>`   | Up to this date                     |
| `--include-merges` | Include merge commits               |
| `--binary`         | Include binary files                |
| `--tui`            | Launch terminal UI                  |
| `-h, --help`       | Show help                           |
| `-V, --version`    | Show version info                   |

Date values support:

`--since` and `--until` accept any of:

* Exact date in `YYYY-MM-DD` (e.g. `2024-01-01`)
* RFC3339 datetime (e.g. `2024-01-01T00:00:00Z`)
* Relative time: 
  - `X days ago`
  - `X weeks ago`
  - `X months ago`
* Git revisions (e.g. `HEAD~10`, `abcdef1`, `main`, or any valid commit, branch, or tag)

> Note: `1 year ago`, `yesterday`, `last month`, and natural language like `today` or `now` are **not supported**.
---
