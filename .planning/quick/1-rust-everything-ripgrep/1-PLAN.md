---
phase: quick
plan: 1
type: execute
wave: 1
depends_on: []
files_modified:
  - Cargo.toml
  - src/main.rs
  - src/lib.rs
  - src/cli.rs
  - src/logging.rs
  - src/error.rs
autonomous: true
requirements: []
---

# Quick Task: Core Project Setup

## Context

This is the initial setup for `ripgrep-soft`, a fast file search tool combining Everything's instant filename search with ripgrep's content search capabilities. This quick task establishes the foundational Rust project structure.

## Task Dependency Graph

| Task | Depends On | Reason |
|------|------------|--------|
| Task 1: Setup Rust project structure | None | Starting point - no prerequisites |

## Parallel Execution Graph

Wave 1 (Start immediately):
- Task 1: Setup Rust project structure (no dependencies)

## Tasks

### Task 1: Setup Rust Project Structure

**Description**: Create the foundational Rust project with CLI framework, logging, and error handling.

**Delegation Recommendation**:
- Category: `deep` - Full-stack implementation with multiple components
- Skills: [] - Standard Rust patterns, no special skills needed

**Skills Evaluation**:
- INCLUDED `none`: Simple project setup, follows standard Rust patterns

**Files**: 
- `Cargo.toml` - Project manifest with all dependencies
- `src/main.rs` - Binary entry point
- `src/lib.rs` - Library root with module declarations
- `src/cli.rs` - CLI argument parsing with clap
- `src/logging.rs` - Tracing-based logging setup
- `src/error.rs` - Custom error types with thiserror

**Action**:
Create a Rust project with the following structure:

1. **Cargo.toml**: Add dependencies:
   - `clap` (4.x) for CLI parsing
   - `tracing` and `tracing-subscriber` for logging
   - `thiserror` for error handling
   - `anyhow` for context-rich errors

2. **src/error.rs**: Define `AppError` enum with variants:
   - `Io(std::io::Error)`
   - `Parse(String)`
   - `Index(String)`

3. **src/logging.rs**: Setup tracing subscriber with:
   - File appender to `logs/app.log`
   - Console output for errors
   - Info level by default

4. **src/cli.rs**: Define CLI with clap:
   - Subcommands: `search`, `index`, `help`
   - `search` args: `--path`, `--pattern`, `--content`, `--regex`
   - Global args: `--verbose`, `--quiet`

5. **src/lib.rs**: Module declarations and re-exports

6. **src/main.rs**: Entry point that:
   - Initializes logging
   - Parses CLI args
   - Routes to subcommand handlers
   - Handles errors gracefully

**Verify**:
```bash
cargo build --release
cargo run -- --help
```

**Done**:
- `cargo build` succeeds without errors
- `cargo run -- --help` displays CLI help
- Logging outputs to console and file on run

## Commit Strategy

Single commit: `init: setup core project structure with CLI, logging, error handling`

## Success Criteria

1. Project compiles: `cargo build` passes
2. CLI works: `--help` shows available commands
3. Basic structure in place for Phase 2 (File Indexing)
