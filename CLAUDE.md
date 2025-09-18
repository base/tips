# TIPS - Transaction Inclusion Prioritization Stack

## Overview
TIPS is an experimental project to replace the p2p mempool with a collection of stateless servies to enable:

- Higher throughput
- Simulation of all transaction
- Cost savings on hardware
- Bundle support

## Code Style & Standards
- Do not add comments unless instructed
- Put imports at the top of the file, never in functions
- Use `just fix` to fix formatting and warnings
- Run `just ci` to verify your changes
- Add dependencies to the Cargo.toml in the root and reference them in the crate cargo files
- Always use the latest dependency versions. Use https://crates.io/ to find dependency versions when adding new deps
- For logging use the tracing crate with appropriate levels and structured logging