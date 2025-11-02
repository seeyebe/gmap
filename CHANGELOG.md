# Changelog

All notable changes to this project will be documented in this file.

## 0.4.0

- Diff accuracy: use robust line-based diff via `similar` crate.
- Cache ordering: commit stats now sorted by timestamp, not commit ID.
- De-duplication: unified cache/repo fetch logic across commands.
- TUI: fixed clipboard support on macOS.
- Merges: `--include-merges` is now opt-in (default: off).
- Minor UX: transient status messages expire cleanly.
- New: Monthly grouping for heatmap via `--monthly`.
- New: Author filters `--author` / `--author-email` across heat/churn/export and TUI.
- New: TUI Files view showing file-type breakdown per period and overall.
- Improved: Natural date parsing supports `yesterday`, `today`, `now`, `last week`, `last month`.
- Misc: Help and README updated.

- New: Default TUI window — last 12 months (monthly) or 52 weeks (weekly); press `A` to toggle “Show All”.
- New: Exclude patterns via `--exclude` and real `.gitignore` honoring (pure Rust via `ignore` crate; no git subprocess).
- Change: Removed Files tab from TUI (kept core tabs: Heatmap, Stats, Timeline, Commits).
- CI: Added GitHub Actions to build and test on push/PR.
