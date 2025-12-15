# Contributing to rtree

## Getting Started

### Prerequisites

- Rust 1.85+ (2024 edition)
- Cargo

### Development Setup

```sh
git clone https://github.com/mattjmcnaughton/rtree.git
cd rtree
cargo build
```

## Project Structure

```
src/
|-- main.rs          # Entry point, CLI argument handling, output orchestration
|-- cli.rs           # Clap CLI argument definitions
|-- lib.rs           # Library root, exports public modules
|-- core/
|   |-- mod.rs       # Core module exports
|   |-- walk.rs      # Async directory traversal logic
|   `-- render.rs    # ASCII tree scaffold rendering
|-- fs/
|   |-- mod.rs       # FileSystem trait definition
|   |-- real.rs      # Real filesystem implementation (tokio-based)
|   `-- mock.rs      # Mock filesystem for testing
`-- models/
    |-- mod.rs       # Model exports
    |-- entry.rs     # FsEntry and EntryKind types
    `-- tree.rs      # TreeNode and DirTree types
```

## Architecture

### Key Design Decisions

**Async filesystem operations**: Uses Tokio for non-blocking directory reads. While the current implementation is sequential, this foundation enables future parallelization.

**Filesystem abstraction**: The `FileSystem` trait (`src/fs/mod.rs`) abstracts filesystem operations, enabling:
- Unit testing with `MockFileSystem`
- Potential future support for remote filesystems or archives

**Separation of concerns**:
- `walk.rs`: Builds the in-memory tree structure
- `render.rs`: Converts the tree to ASCII output
- `models/`: Pure data structures with no behavior

**Symlink handling**: Symlinks are treated as leaf nodes and never followed, preventing infinite loops from circular symlinks.

### Data Flow

1. `main.rs` parses CLI arguments and validates the root path
2. `walk_dir()` recursively traverses the filesystem, building a `DirTree`
3. `write_children()` renders the tree to stdout with ASCII scaffold

## Building and Testing

### Build

```sh
# Debug build
cargo build

# Release build (optimized)
cargo build --release
```

### Run Tests

```sh
# Run all tests
cargo test

# Run tests with output
cargo test -- --nocapture
```

### Run Locally

```sh
# Via cargo
cargo run -- /path/to/dir

# Direct binary
./target/debug/rtree /path/to/dir
```

## Code Style

### Formatting

```sh
cargo fmt
```

### Linting

```sh
cargo clippy
```

## Making Changes

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/my-feature`
3. Make your changes
4. Ensure tests pass: `cargo test`
5. Format code: `cargo fmt`
6. Run clippy: `cargo clippy`
7. Commit your changes
8. Open a pull request

### Adding New Features

The project follows a specification-driven approach. See `spec.md` for the current v0 specification. Features marked as "non-goals" in the spec are intentionally deferred for future versions.

### Writing Tests

- Unit tests live alongside the code they test (in `#[cfg(test)]` modules)
- Use `MockFileSystem` for testing filesystem-dependent code
- Tests should be deterministic and not depend on real filesystem state
