# rtree

A fast, deterministic directory tree visualization tool written in Rust. A clone of the classic Unix `tree` command with async filesystem traversal.

## Features

- **Deterministic output**: Repeated runs over the same filesystem state produce identical output
- **Human-friendly**: ASCII tree scaffold clearly communicates directory nesting
- **Complete traversal**: Displays all visible entries including dotfiles
- **Robust error handling**: Permission errors are reported inline without crashing
- **Symlink-safe**: Symlinks are displayed but not followed, preventing cycles

## Installation

WIP (eventually will be homebrew)

## Usage

```sh
# Display tree for current directory
rtree

# Display tree for a specific path
rtree /path/to/directory

# Display help
rtree --help
```

### Example Output

```
.
|-- Cargo.lock
|-- Cargo.toml
|-- README.md
`-- src/
    |-- cli.rs
    |-- core/
    |   |-- mod.rs
    |   |-- render.rs
    |   `-- walk.rs
    |-- fs/
    |   |-- mock.rs
    |   |-- mod.rs
    |   `-- real.rs
    |-- lib.rs
    |-- main.rs
    `-- models/
        |-- entry.rs
        |-- mod.rs
        `-- tree.rs
```

## Output Format

### ASCII Connectors

- `|-- ` prefix for entries with siblings following
- `` `-- `` prefix for the last entry in a directory
- `|   ` vertical continuation when ancestor directories have more siblings
- `    ` (4 spaces) when ancestor directory was the last entry

### Naming Conventions

- Directories are suffixed with `/`
- Entries are sorted by name (byte/Unicode codepoint order)
- Files and directories are interleaved in sort order

### Error Handling

When a directory cannot be read, the error is displayed inline:

```
`-- secret/ [error: Permission denied]
```

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for development setup, project structure, and contribution guidelines.

## License

GPL v2
