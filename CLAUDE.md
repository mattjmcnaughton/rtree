# CLAUDE.md / AGENTS.md

This file provides guidance to Claude Code (claude.ai/code) and other AI agents when working with code in this repository.

## Build Commands

```sh
cargo build              # Debug build
cargo build --release    # Release build
cargo test               # Run all tests
cargo test -- --nocapture  # Run tests with output
cargo fmt                # Format code
cargo clippy             # Lint
cargo run -- /path/to/dir  # Run locally
```

## Architecture

rtree is a Rust CLI that renders directory trees with ASCII scaffolding. It uses async/await with Tokio for filesystem operations.

### Module Structure

- **`src/main.rs`**: Entry point, CLI parsing, output orchestration
- **`src/cli.rs`**: Clap argument definitions
- **`src/core/walk.rs`**: Async directory traversal, builds `DirTree`
- **`src/core/render.rs`**: ASCII tree scaffold rendering
- **`src/fs/mod.rs`**: `FileSystem` trait abstracting filesystem operations
- **`src/fs/real.rs`**: Real tokio-based filesystem implementation
- **`src/fs/mock.rs`**: Mock filesystem for testing
- **`src/models/entry.rs`**: `FsEntry` and `EntryKind` types
- **`src/models/tree.rs`**: `TreeNode` and `DirTree` types

### Data Flow

1. CLI parses arguments and validates root path
2. `walk_dir()` recursively traverses filesystem, building a `DirTree`
3. `write_children()` renders tree to stdout with ASCII scaffold

### Key Design Patterns

- **Filesystem trait abstraction**: Use `MockFileSystem` for tests instead of touching real files
- **Symlinks as leaf nodes**: Never followed, preventing cycles
- **Deterministic output**: Entries sorted by name for reproducible results

## Rust CLI Best Practices

### Currently Implemented

**Error Handling**
- Use `anyhow::Result` for application errors
- Return errors rather than panicking; reserve `unwrap()`/`expect()` for tests only
- Provide actionable error messages that include the path and underlying error

**CLI Design**
- Use `clap` with derive macros for argument parsing
- Exit with appropriate codes: 0 for success, non-zero for errors
- Write normal output to stdout, errors/warnings to stderr

**Testing**
- Abstract I/O behind traits for testability (like `FileSystem` trait)
- Use `MockFileSystem` for unit tests instead of real filesystem
- Test error paths, not just happy paths
- Unit tests live alongside code in `#[cfg(test)]` modules

**Code Organization**
- Keep `main.rs` thin: parse args, call library code, handle top-level errors
- Put business logic in library modules so it can be tested and reused
- Group related functionality into modules with clear boundaries

### Planned Improvements (as of 0f37c82)

- Add `.context()` / `.with_context()` to errors for better debugging
- Use `BufWriter` for stdout when writing many small pieces
- Add e2e tests in `tests/` that exercise the actual CLI binary
