# Project: Tips - Transaction Inclusion Pipeline Services

## Notes
- Always run `just ci` before claiming a task is complete and fix any issues
- Use `just fix` to fix formatting and warnings
- Only add comments when the implementation logic is unclear, i.e. do not comment insert item into database when the code is db.insert(item)
- Always add dependencies to the cargo.toml in the root and reference them in the crate cargo files
- Use https://crates.io/ to find dependency versions when adding new deps

## Project Structure
```
├── Cargo.toml          # Workspace configuration
├── ingress/            # Main binary crate
├── .github/workflows/
│   └── ci.yml         # GitHub Actions CI
```
