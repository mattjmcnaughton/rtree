# rtree

A fast, deterministic directory tree visualization tool written in Rust. A clone of the classic Unix `tree` command with async filesystem traversal.

## Features

- **Deterministic output**: Repeated runs over the same filesystem state produce identical output
- **Human-friendly**: ASCII tree scaffold clearly communicates directory nesting
- **Complete traversal**: Displays all visible entries including dotfiles
- **Flexible filtering**: Limit depth with `-L`, exclude patterns with `-I`, show directories only with `-d`
- **Customizable sorting**: Use `--dirsfirst` to list directories before files
- **Robust error handling**: Permission errors are reported inline without crashing
- **Symlink-safe**: Symlinks are displayed but not followed, preventing cycles

## Installation

### Homebrew

```sh
brew tap mattjmcnaughton/tap
brew install mattjmcnaughton/tap/rtree
```

### Build from Source

```sh
git clone https://github.com/mattjmcnaughton/rtree.git
cd rtree
cargo build --release
# Binary will be at target/release/rtree
```

## Usage

```sh
# Display tree for current directory
rtree

# Display tree for a specific path
rtree /path/to/directory

# Display help
rtree --help
```

### Options

| Flag | Description |
|------|-------------|
| `-L <depth>` | Limit display to `<depth>` levels of directories |
| `-I <pattern>` | Exclude files/directories matching pattern (pipe-separated, supports `*` and `?` globs, e.g., `*.log\|node_modules`) |
| `-d` | List directories only |
| `--dirsfirst` | List directories before files |
| `-a` | Show all files (default behavior, included for tree compatibility) |

### Examples

```sh
# Show only 2 levels deep
rtree -L 2

# Ignore node_modules and dist directories
rtree -I "node_modules|dist"

# Ignore all .log files using glob pattern
rtree -I "*.log"

# Show only directories, sorted before files
rtree -d --dirsfirst

# Combine options: 3 levels, ignore .git, dirs first
rtree -L 3 -I .git --dirsfirst
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

## Acknowledgments

This project is inspired by and aims to be compatible with the classic Unix [`tree`](https://github.com/Old-Man-Programmer/tree) command by Steve Baker.

## License

GPL v2
