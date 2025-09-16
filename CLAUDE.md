# Project: Tips - Transaction Inclusion Pipeline Services

## Notes
- DO NOT ADD COMMENTS UNLESS INSTRUCTED
- Put imports at the top of the file, never in functions
- Always run `just ci` before claiming a task is complete and fix any issues
- Use `just fix` to fix formatting and warnings
- Always add dependencies to the cargo.toml in the root and reference them in the crate cargo files
- Always use the latest dependency versions. Use https://crates.io/ to find dependency versions when adding new deps

## Available CI Commands
- `just ci` - Run all CI checks (both Rust and UI)
- `just rust-ci` - Run Rust-specific CI checks (check, test, fmt, clippy, build)
- `just ui-ci` - Run UI-specific CI checks

## Project Structure
```
├── Cargo.toml          # Workspace configuration
├── ingress/            # Main binary crate
├── .github/workflows/
│   └── ci.yml         # GitHub Actions CI
```
