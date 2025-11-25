# gmap

Git activity maps you can use: heatmaps, churn, exports, and a no‑nonsense TUI.

No dashboards. No hype. Just answers to: what changed, when, and by whom.

## Install

```sh
cargo install gmap
```

Requires a Git repo. Data is read locally; nothing leaves your machine.

## Quick start

- TUI heatmap for the current repo
  ```sh
  gmap heat --tui
  ```

- JSON heatmap, monthly buckets, filter by author
  ```sh
  gmap --repo /path/to/repo heat --json --monthly --author "alice"
  ```

- Top churn (aggregate file changes)
  ```sh
  gmap churn --json --since 3 months ago
  ```

- Full export (per‑commit, per‑file), newline‑delimited
  ```sh
  gmap export --ndjson
  ```

## Flags you’ll actually use

- Global
  - `--repo <path>`: analyze this repo (default: `.`)
  - `--cache <dir>`: where to place/read the `.db` cache
  - `--since/--until <date>`: time bounds (supports `YYYY-MM-DD`, RFC3339, `X days/weeks/months ago`, `yesterday`, `today`, `now`, `last week`, `last month`, or any Git ref)
  - `--include-merges`: count merge commits (off by default)
  - `--author <text>` / `--author-email <text>`: substring match, case‑insensitive

- Heat
  - `--json` / `--ndjson`
  - `--monthly`: group by month instead of week

- Churn
  - `--json` / `--ndjson`
  - `--depth <n>`: aggregate by directory depth

- Export
  - `--json` / `--ndjson`

## TUI

- Tabs: Heatmap • Stats • Files • Timeline • Commits
- Keys
  - `Tab` / `Shift+Tab`: switch views
  - `←/→` or `j/k`: move selection
  - `/`: search weeks/authors (filter)
  - `Enter`: open commit list for selected period
  - `c`: copy commit hash
  - `h` or `F1`: help; `q`: quit

Tip: The Files view shows file‑type breakdowns for the selected period and overall, so you can spot what kinds of files are being touched.

## What gmap gives you

- A timeline of change: commits and line deltas by week/month
- Churn hotspots: where change concentrates (by file or directory)
- Contributors and patterns: filter by author/email to slice activity
- Portable data: JSON/NDJSON outputs for your own scripts

What it’s not: a general Git UI, blame replacement, or a web app.

## Performance notes

- Uses a local SQLite cache under `.gmap/` to avoid recomputing diffs
- Line diffs use a robust algorithm; binary files are ignored unless `--binary`
- Merge commits are excluded by default to reduce noise

## Testing

- `cargo test` runs CLI smoke tests that spawn temporary git repos. On Windows, set `git config --global core.autocrlf false` (or `input`) to avoid CRLF churn blocking branch checkouts. The tests hard-reset their temp repos between branch switches to keep worktrees clean.

## Contributing

Bug reports, ideas, and small PRs welcome. Keep changes focused. If you add a feature, add a flag and a short example to this README.

## License

MIT. See `LICENSE`.
