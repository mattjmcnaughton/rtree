use std::collections::HashSet;
use std::path::Path;

use anyhow::Context;
use regex::RegexSet;

use crate::fs::FileSystem;
use crate::models::{DirTree, EntryKind, TreeNode};

/// Pre-compiled ignore patterns for efficient matching.
/// Separates exact-match patterns from glob patterns for optimal performance.
pub struct CompiledPatterns {
    /// Patterns without wildcards - use fast exact matching
    exact_matches: HashSet<String>,
    /// Compiled regex set for glob patterns with wildcards
    regex_set: Option<RegexSet>,
}

impl CompiledPatterns {
    /// Compile a pipe-separated pattern string into efficient matchers.
    /// Returns an error if any glob pattern produces invalid regex.
    pub fn new(pattern: &str) -> anyhow::Result<Self> {
        let mut exact_matches = HashSet::new();
        let mut regex_patterns = Vec::new();

        for segment in pattern.split('|') {
            let p = segment.trim();
            if p.is_empty() {
                continue;
            }

            if p.contains('*') || p.contains('?') {
                // Glob pattern - needs regex
                let regex_str = glob_to_regex(p);
                regex_patterns.push(regex_str);
            } else {
                // Exact match - fast path
                exact_matches.insert(p.to_owned());
            }
        }

        let regex_set = if regex_patterns.is_empty() {
            None
        } else {
            Some(
                RegexSet::new(&regex_patterns)
                    .with_context(|| format!("Invalid ignore pattern: {pattern}"))?,
            )
        };

        Ok(Self {
            exact_matches,
            regex_set,
        })
    }

    /// Check if a name matches any of the compiled patterns.
    #[inline]
    pub fn matches(&self, name: &str) -> bool {
        // Fast path: exact match check (O(1) HashSet lookup)
        if self.exact_matches.contains(name) {
            return true;
        }

        // Slow path: regex matching
        if let Some(ref regex_set) = self.regex_set {
            return regex_set.is_match(name);
        }

        false
    }
}

/// Convert a glob pattern to a regex string.
/// Supports `*` (any sequence) and `?` (single char) wildcards.
fn glob_to_regex(pattern: &str) -> String {
    let mut regex_pattern = String::with_capacity(pattern.len() * 2 + 2);
    regex_pattern.push('^');

    for c in pattern.chars() {
        match c {
            '*' => regex_pattern.push_str(".*"),
            '?' => regex_pattern.push('.'),
            // Escape regex special characters
            '.' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '\\' | '|' => {
                regex_pattern.push('\\');
                regex_pattern.push(c);
            }
            _ => regex_pattern.push(c),
        }
    }

    regex_pattern.push('$');
    regex_pattern
}

/// Configuration options for directory traversal.
#[derive(Debug, Clone)]
pub struct WalkOptions {
    /// Maximum depth to traverse (None means unlimited)
    pub max_depth: Option<usize>,
    /// Pipe-separated patterns to ignore (e.g., "node_modules|.git|dist")
    pub ignore_pattern: Option<String>,
    /// Whether to show hidden files (starting with '.')
    pub show_hidden: bool,
    /// Whether to show only directories
    pub dirs_only: bool,
    /// Whether to sort directories before files
    pub dirs_first: bool,
}

impl Default for WalkOptions {
    fn default() -> Self {
        Self {
            max_depth: None,
            ignore_pattern: None,
            show_hidden: true, // Current behavior: show hidden files by default
            dirs_only: false,
            dirs_first: false,
        }
    }
}

/// Walk a directory tree with the given options.
///
/// This is the public entry point that starts traversal at depth 0.
/// Returns an error if the ignore pattern is invalid.
pub async fn walk_dir<F: FileSystem>(
    fs: &F,
    dir: &Path,
    options: &WalkOptions,
) -> anyhow::Result<DirTree> {
    // Pre-compile patterns once before traversal
    let compiled_patterns = match &options.ignore_pattern {
        Some(pattern) => Some(CompiledPatterns::new(pattern)?),
        None => None,
    };

    Ok(walk_dir_internal(fs, dir, options, &compiled_patterns, 0).await)
}

/// Internal recursive function that tracks current depth.
async fn walk_dir_internal<F: FileSystem>(
    fs: &F,
    dir: &Path,
    options: &WalkOptions,
    compiled_patterns: &Option<CompiledPatterns>,
    current_depth: usize,
) -> DirTree {
    let entries = match fs.read_dir(dir).await {
        Ok(entries) => entries,
        Err(err) => {
            return DirTree {
                error: Some(err.to_string()),
                children: Vec::new(),
            };
        }
    };

    // Filter entries based on options
    let filtered_entries: Vec<_> = entries
        .into_iter()
        .filter(|entry| {
            // Filter hidden files if show_hidden is false
            if !options.show_hidden && entry.name.starts_with('.') {
                return false;
            }

            // Filter by compiled ignore patterns
            if let Some(patterns) = compiled_patterns
                && patterns.matches(&entry.name)
            {
                return false;
            }

            // Filter non-directories if dirs_only is true
            if options.dirs_only && entry.kind != EntryKind::Directory {
                return false;
            }

            true
        })
        .collect();

    let mut entries_with_rendered: Vec<(String, _)> = filtered_entries
        .into_iter()
        .map(|entry| (rendered_name(&entry.name, entry.kind), entry))
        .collect();

    // Sort entries: dirs-first if enabled, then alphabetically by rendered name
    if options.dirs_first {
        entries_with_rendered.sort_by(|(name_a, entry_a), (name_b, entry_b)| {
            match (entry_a.kind, entry_b.kind) {
                (EntryKind::Directory, EntryKind::Directory) => name_a.cmp(name_b),
                (EntryKind::Directory, _) => std::cmp::Ordering::Less,
                (_, EntryKind::Directory) => std::cmp::Ordering::Greater,
                _ => name_a.cmp(name_b),
            }
        });
    } else {
        entries_with_rendered.sort_by(|(a, _), (b, _)| a.cmp(b));
    }

    let mut children = Vec::with_capacity(entries_with_rendered.len());
    for (rendered, entry) in entries_with_rendered {
        let mut node = TreeNode {
            name: rendered,
            kind: entry.kind,
            error: None,
            children: Vec::new(),
        };

        // Only recurse into directories if we haven't reached max depth
        // Note: -L 1 means "show 1 level of children", so at depth 0 we should not recurse
        if entry.kind == EntryKind::Directory {
            let should_recurse = match options.max_depth {
                Some(max) => current_depth + 1 < max,
                None => true,
            };

            if should_recurse {
                let subtree = Box::pin(walk_dir_internal(
                    fs,
                    &entry.path,
                    options,
                    compiled_patterns,
                    current_depth + 1,
                ))
                .await;
                node.error = subtree.error;
                node.children = subtree.children;
            }
        }

        children.push(node);
    }

    DirTree {
        error: None,
        children,
    }
}

fn rendered_name(name: &str, kind: EntryKind) -> String {
    match kind {
        EntryKind::Directory => format!("{name}/"),
        EntryKind::File | EntryKind::Symlink | EntryKind::Other => name.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::MockFileSystem;
    use crate::models::FsEntry;
    use std::path::PathBuf;

    #[tokio::test]
    async fn sorts_by_rendered_name_including_directory_suffix() {
        let fs = MockFileSystem::default();
        fs.set_dir_entries(
            "/root",
            vec![
                FsEntry {
                    path: PathBuf::from("/root/a-dir"),
                    name: "a".to_owned(),
                    kind: EntryKind::Directory,
                },
                FsEntry {
                    path: PathBuf::from("/root/a-file"),
                    name: "a".to_owned(),
                    kind: EntryKind::File,
                },
                FsEntry {
                    path: PathBuf::from("/root/b"),
                    name: "b".to_owned(),
                    kind: EntryKind::File,
                },
            ],
        );
        fs.set_dir_entries("/root/a-dir", vec![]);

        let options = WalkOptions::default();
        let tree = walk_dir(&fs, Path::new("/root"), &options).await.unwrap();
        let names: Vec<String> = tree.children.into_iter().map(|n| n.name).collect();
        assert_eq!(names, vec!["a".to_owned(), "a/".to_owned(), "b".to_owned()]);
    }

    #[tokio::test]
    async fn unreadable_directory_is_recorded_and_not_descended() {
        let fs = MockFileSystem::default();
        fs.set_dir_entries(
            "/root",
            vec![FsEntry {
                path: PathBuf::from("/root/secret"),
                name: "secret".to_owned(),
                kind: EntryKind::Directory,
            }],
        );
        fs.set_error("/root/secret", "Permission denied");

        let options = WalkOptions::default();
        let tree = walk_dir(&fs, Path::new("/root"), &options).await.unwrap();
        assert_eq!(tree.error, None);
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].name, "secret/".to_owned());
        assert!(
            tree.children[0]
                .error
                .as_deref()
                .unwrap_or("")
                .contains("Permission")
        );
        assert!(tree.children[0].children.is_empty());
    }

    #[tokio::test]
    async fn symlinks_are_leaf_nodes() {
        let fs = MockFileSystem::default();
        fs.set_dir_entries(
            "/root",
            vec![FsEntry {
                path: PathBuf::from("/root/link"),
                name: "link".to_owned(),
                kind: EntryKind::Symlink,
            }],
        );
        fs.set_dir_entries(
            "/root/link",
            vec![FsEntry {
                path: PathBuf::from("/root/link/child"),
                name: "child".to_owned(),
                kind: EntryKind::File,
            }],
        );

        let options = WalkOptions::default();
        let tree = walk_dir(&fs, Path::new("/root"), &options).await.unwrap();
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].name, "link".to_owned());
        assert_eq!(tree.children[0].children.len(), 0);

        let calls: Vec<String> = fs
            .calls()
            .into_iter()
            .map(|p| p.to_string_lossy().into_owned())
            .collect();
        assert_eq!(calls, vec!["/root".to_owned()]);
    }

    // --- Depth limiting tests ---

    #[tokio::test]
    async fn depth_limit_stops_at_specified_level() {
        let fs = MockFileSystem::default();
        fs.set_dir_entries(
            "/root",
            vec![FsEntry {
                path: PathBuf::from("/root/level1"),
                name: "level1".to_owned(),
                kind: EntryKind::Directory,
            }],
        );
        fs.set_dir_entries(
            "/root/level1",
            vec![FsEntry {
                path: PathBuf::from("/root/level1/level2"),
                name: "level2".to_owned(),
                kind: EntryKind::Directory,
            }],
        );
        fs.set_dir_entries(
            "/root/level1/level2",
            vec![FsEntry {
                path: PathBuf::from("/root/level1/level2/level3"),
                name: "level3".to_owned(),
                kind: EntryKind::Directory,
            }],
        );

        // -L 1 should show only immediate children, no recursion
        let options = WalkOptions {
            max_depth: Some(1),
            ..WalkOptions::default()
        };
        let tree = walk_dir(&fs, Path::new("/root"), &options).await.unwrap();
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].name, "level1/");
        assert!(tree.children[0].children.is_empty());

        // -L 2 should show two levels
        let options = WalkOptions {
            max_depth: Some(2),
            ..WalkOptions::default()
        };
        let tree = walk_dir(&fs, Path::new("/root"), &options).await.unwrap();
        assert_eq!(tree.children[0].children.len(), 1);
        assert_eq!(tree.children[0].children[0].name, "level2/");
        assert!(tree.children[0].children[0].children.is_empty());
    }

    #[tokio::test]
    async fn depth_limit_none_traverses_all() {
        let fs = MockFileSystem::default();
        fs.set_dir_entries(
            "/root",
            vec![FsEntry {
                path: PathBuf::from("/root/a"),
                name: "a".to_owned(),
                kind: EntryKind::Directory,
            }],
        );
        fs.set_dir_entries(
            "/root/a",
            vec![FsEntry {
                path: PathBuf::from("/root/a/b"),
                name: "b".to_owned(),
                kind: EntryKind::Directory,
            }],
        );
        fs.set_dir_entries(
            "/root/a/b",
            vec![FsEntry {
                path: PathBuf::from("/root/a/b/c"),
                name: "c".to_owned(),
                kind: EntryKind::File,
            }],
        );

        let options = WalkOptions::default(); // max_depth is None
        let tree = walk_dir(&fs, Path::new("/root"), &options).await.unwrap();
        assert_eq!(tree.children[0].children[0].children.len(), 1);
        assert_eq!(tree.children[0].children[0].children[0].name, "c");
    }

    // --- Ignore pattern tests ---

    #[tokio::test]
    async fn ignore_single_pattern() {
        let fs = MockFileSystem::default();
        fs.set_dir_entries(
            "/root",
            vec![
                FsEntry {
                    path: PathBuf::from("/root/keep"),
                    name: "keep".to_owned(),
                    kind: EntryKind::File,
                },
                FsEntry {
                    path: PathBuf::from("/root/node_modules"),
                    name: "node_modules".to_owned(),
                    kind: EntryKind::Directory,
                },
            ],
        );

        let options = WalkOptions {
            ignore_pattern: Some("node_modules".to_owned()),
            ..WalkOptions::default()
        };
        let tree = walk_dir(&fs, Path::new("/root"), &options).await.unwrap();
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].name, "keep");
    }

    #[tokio::test]
    async fn ignore_pipe_separated_patterns() {
        let fs = MockFileSystem::default();
        fs.set_dir_entries(
            "/root",
            vec![
                FsEntry {
                    path: PathBuf::from("/root/keep"),
                    name: "keep".to_owned(),
                    kind: EntryKind::File,
                },
                FsEntry {
                    path: PathBuf::from("/root/node_modules"),
                    name: "node_modules".to_owned(),
                    kind: EntryKind::Directory,
                },
                FsEntry {
                    path: PathBuf::from("/root/dist"),
                    name: "dist".to_owned(),
                    kind: EntryKind::Directory,
                },
                FsEntry {
                    path: PathBuf::from("/root/.git"),
                    name: ".git".to_owned(),
                    kind: EntryKind::Directory,
                },
            ],
        );

        let options = WalkOptions {
            ignore_pattern: Some("node_modules|dist|.git".to_owned()),
            ..WalkOptions::default()
        };
        let tree = walk_dir(&fs, Path::new("/root"), &options).await.unwrap();
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].name, "keep");
    }

    // --- Dirs only tests ---

    #[tokio::test]
    async fn dirs_only_excludes_files() {
        let fs = MockFileSystem::default();
        fs.set_dir_entries(
            "/root",
            vec![
                FsEntry {
                    path: PathBuf::from("/root/dir"),
                    name: "dir".to_owned(),
                    kind: EntryKind::Directory,
                },
                FsEntry {
                    path: PathBuf::from("/root/file.txt"),
                    name: "file.txt".to_owned(),
                    kind: EntryKind::File,
                },
            ],
        );
        fs.set_dir_entries("/root/dir", vec![]);

        let options = WalkOptions {
            dirs_only: true,
            ..WalkOptions::default()
        };
        let tree = walk_dir(&fs, Path::new("/root"), &options).await.unwrap();
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].name, "dir/");
    }

    #[tokio::test]
    async fn dirs_only_excludes_symlinks() {
        let fs = MockFileSystem::default();
        fs.set_dir_entries(
            "/root",
            vec![
                FsEntry {
                    path: PathBuf::from("/root/dir"),
                    name: "dir".to_owned(),
                    kind: EntryKind::Directory,
                },
                FsEntry {
                    path: PathBuf::from("/root/link"),
                    name: "link".to_owned(),
                    kind: EntryKind::Symlink,
                },
            ],
        );
        fs.set_dir_entries("/root/dir", vec![]);

        let options = WalkOptions {
            dirs_only: true,
            ..WalkOptions::default()
        };
        let tree = walk_dir(&fs, Path::new("/root"), &options).await.unwrap();
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].name, "dir/");
    }

    // --- Dirs first tests ---

    #[tokio::test]
    async fn dirs_first_sorts_directories_before_files() {
        let fs = MockFileSystem::default();
        fs.set_dir_entries(
            "/root",
            vec![
                FsEntry {
                    path: PathBuf::from("/root/zebra.txt"),
                    name: "zebra.txt".to_owned(),
                    kind: EntryKind::File,
                },
                FsEntry {
                    path: PathBuf::from("/root/alpha"),
                    name: "alpha".to_owned(),
                    kind: EntryKind::Directory,
                },
                FsEntry {
                    path: PathBuf::from("/root/beta.txt"),
                    name: "beta.txt".to_owned(),
                    kind: EntryKind::File,
                },
            ],
        );
        fs.set_dir_entries("/root/alpha", vec![]);

        let options = WalkOptions {
            dirs_first: true,
            ..WalkOptions::default()
        };
        let tree = walk_dir(&fs, Path::new("/root"), &options).await.unwrap();
        let names: Vec<&str> = tree.children.iter().map(|n| n.name.as_str()).collect();
        // Directory first, then files alphabetically
        assert_eq!(names, vec!["alpha/", "beta.txt", "zebra.txt"]);
    }

    #[tokio::test]
    async fn dirs_first_maintains_alphabetical_within_category() {
        let fs = MockFileSystem::default();
        fs.set_dir_entries(
            "/root",
            vec![
                FsEntry {
                    path: PathBuf::from("/root/zdir"),
                    name: "zdir".to_owned(),
                    kind: EntryKind::Directory,
                },
                FsEntry {
                    path: PathBuf::from("/root/adir"),
                    name: "adir".to_owned(),
                    kind: EntryKind::Directory,
                },
                FsEntry {
                    path: PathBuf::from("/root/zfile"),
                    name: "zfile".to_owned(),
                    kind: EntryKind::File,
                },
                FsEntry {
                    path: PathBuf::from("/root/afile"),
                    name: "afile".to_owned(),
                    kind: EntryKind::File,
                },
            ],
        );
        fs.set_dir_entries("/root/adir", vec![]);
        fs.set_dir_entries("/root/zdir", vec![]);

        let options = WalkOptions {
            dirs_first: true,
            ..WalkOptions::default()
        };
        let tree = walk_dir(&fs, Path::new("/root"), &options).await.unwrap();
        let names: Vec<&str> = tree.children.iter().map(|n| n.name.as_str()).collect();
        // Dirs alphabetically, then files alphabetically
        assert_eq!(names, vec!["adir/", "zdir/", "afile", "zfile"]);
    }

    // --- Hidden files tests ---

    #[tokio::test]
    async fn hidden_files_shown_by_default() {
        let fs = MockFileSystem::default();
        fs.set_dir_entries(
            "/root",
            vec![
                FsEntry {
                    path: PathBuf::from("/root/.hidden"),
                    name: ".hidden".to_owned(),
                    kind: EntryKind::File,
                },
                FsEntry {
                    path: PathBuf::from("/root/visible"),
                    name: "visible".to_owned(),
                    kind: EntryKind::File,
                },
            ],
        );

        let options = WalkOptions::default(); // show_hidden is true
        let tree = walk_dir(&fs, Path::new("/root"), &options).await.unwrap();
        assert_eq!(tree.children.len(), 2);
    }

    #[tokio::test]
    async fn hidden_files_excluded_when_show_hidden_false() {
        let fs = MockFileSystem::default();
        fs.set_dir_entries(
            "/root",
            vec![
                FsEntry {
                    path: PathBuf::from("/root/.hidden"),
                    name: ".hidden".to_owned(),
                    kind: EntryKind::File,
                },
                FsEntry {
                    path: PathBuf::from("/root/.gitignore"),
                    name: ".gitignore".to_owned(),
                    kind: EntryKind::File,
                },
                FsEntry {
                    path: PathBuf::from("/root/visible"),
                    name: "visible".to_owned(),
                    kind: EntryKind::File,
                },
            ],
        );

        let options = WalkOptions {
            show_hidden: false,
            ..WalkOptions::default()
        };
        let tree = walk_dir(&fs, Path::new("/root"), &options).await.unwrap();
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].name, "visible");
    }

    // --- Combined options tests ---

    #[tokio::test]
    async fn combined_dirs_first_and_ignore_pattern() {
        let fs = MockFileSystem::default();
        fs.set_dir_entries(
            "/root",
            vec![
                FsEntry {
                    path: PathBuf::from("/root/src"),
                    name: "src".to_owned(),
                    kind: EntryKind::Directory,
                },
                FsEntry {
                    path: PathBuf::from("/root/node_modules"),
                    name: "node_modules".to_owned(),
                    kind: EntryKind::Directory,
                },
                FsEntry {
                    path: PathBuf::from("/root/README.md"),
                    name: "README.md".to_owned(),
                    kind: EntryKind::File,
                },
            ],
        );
        fs.set_dir_entries("/root/src", vec![]);

        let options = WalkOptions {
            dirs_first: true,
            ignore_pattern: Some("node_modules".to_owned()),
            ..WalkOptions::default()
        };
        let tree = walk_dir(&fs, Path::new("/root"), &options).await.unwrap();
        let names: Vec<&str> = tree.children.iter().map(|n| n.name.as_str()).collect();
        // node_modules filtered out, src/ first, then README.md
        assert_eq!(names, vec!["src/", "README.md"]);
    }

    #[tokio::test]
    async fn combined_depth_limit_and_dirs_only() {
        let fs = MockFileSystem::default();
        fs.set_dir_entries(
            "/root",
            vec![
                FsEntry {
                    path: PathBuf::from("/root/dir1"),
                    name: "dir1".to_owned(),
                    kind: EntryKind::Directory,
                },
                FsEntry {
                    path: PathBuf::from("/root/file1"),
                    name: "file1".to_owned(),
                    kind: EntryKind::File,
                },
            ],
        );
        fs.set_dir_entries(
            "/root/dir1",
            vec![
                FsEntry {
                    path: PathBuf::from("/root/dir1/subdir"),
                    name: "subdir".to_owned(),
                    kind: EntryKind::Directory,
                },
                FsEntry {
                    path: PathBuf::from("/root/dir1/file2"),
                    name: "file2".to_owned(),
                    kind: EntryKind::File,
                },
            ],
        );
        fs.set_dir_entries("/root/dir1/subdir", vec![]);

        let options = WalkOptions {
            max_depth: Some(2),
            dirs_only: true,
            ..WalkOptions::default()
        };
        let tree = walk_dir(&fs, Path::new("/root"), &options).await.unwrap();
        // Only directories shown, depth limited to 2 levels
        assert_eq!(tree.children.len(), 1);
        assert_eq!(tree.children[0].name, "dir1/");
        assert_eq!(tree.children[0].children.len(), 1);
        assert_eq!(tree.children[0].children[0].name, "subdir/");
    }

    // --- CompiledPatterns tests ---

    #[test]
    fn compiled_patterns_exact_match() {
        let patterns = CompiledPatterns::new("node_modules").unwrap();
        assert!(patterns.matches("node_modules"));
        assert!(!patterns.matches("node_modules_extra"));
        assert!(!patterns.matches("my_node_modules"));
    }

    #[test]
    fn compiled_patterns_pipe_separated() {
        let patterns = CompiledPatterns::new("node_modules|dist|.git").unwrap();
        assert!(patterns.matches("dist"));
        assert!(patterns.matches(".git"));
        assert!(!patterns.matches("src"));
    }

    #[test]
    fn compiled_patterns_handles_whitespace() {
        let patterns = CompiledPatterns::new("node_modules | dist | .git").unwrap();
        assert!(patterns.matches("dist"));
        let patterns2 = CompiledPatterns::new("  .git  ").unwrap();
        assert!(patterns2.matches(".git"));
    }

    #[test]
    fn compiled_patterns_empty_segments_ignored() {
        let patterns = CompiledPatterns::new("node_modules||dist").unwrap();
        assert!(!patterns.matches(""));
        assert!(patterns.matches("dist"));
    }

    // --- Glob pattern tests ---

    #[test]
    fn compiled_patterns_star_wildcard() {
        // * matches any sequence
        let patterns = CompiledPatterns::new("*.log").unwrap();
        assert!(patterns.matches("test.log"));
        assert!(patterns.matches("app.log"));
        assert!(patterns.matches(".log")); // empty prefix
        assert!(!patterns.matches("test.txt"));

        // * at end
        let patterns = CompiledPatterns::new("test_*").unwrap();
        assert!(patterns.matches("test_foo"));
        assert!(patterns.matches("test_")); // empty suffix
        assert!(!patterns.matches("other_foo"));

        // * in middle
        let patterns = CompiledPatterns::new("test_*_bar").unwrap();
        assert!(patterns.matches("test_foo_bar"));
        assert!(patterns.matches("test__bar")); // empty middle
        assert!(!patterns.matches("test_foo_baz"));

        // multiple stars
        let patterns = CompiledPatterns::new("*_*_*").unwrap();
        assert!(patterns.matches("a_b_c"));
        assert!(patterns.matches("__"));
    }

    #[test]
    fn compiled_patterns_question_wildcard() {
        // ? matches exactly one character
        let patterns = CompiledPatterns::new("?.txt").unwrap();
        assert!(patterns.matches("a.txt"));
        assert!(patterns.matches("b.txt"));
        assert!(!patterns.matches("ab.txt"));
        assert!(!patterns.matches(".txt"));

        // multiple ?
        let patterns = CompiledPatterns::new("??.txt").unwrap();
        assert!(patterns.matches("ab.txt"));
        assert!(!patterns.matches("a.txt"));
        assert!(!patterns.matches("abc.txt"));
    }

    #[test]
    fn compiled_patterns_combined_wildcards() {
        // * and ? together
        let patterns = CompiledPatterns::new("test?.log").unwrap();
        assert!(patterns.matches("test1.log"));
        assert!(patterns.matches("test2.log"));
        assert!(!patterns.matches("test12.log"));

        let patterns = CompiledPatterns::new("file*.*").unwrap();
        assert!(patterns.matches("file1.txt"));
        assert!(patterns.matches("file123.txt"));
        assert!(patterns.matches("file.txt"));
    }

    #[test]
    fn compiled_patterns_escapes_regex_special_chars() {
        // Dots should be literal
        let patterns = CompiledPatterns::new("test.txt").unwrap();
        assert!(patterns.matches("test.txt"));
        assert!(!patterns.matches("testXtxt"));

        // Other regex chars should be literal
        let patterns = CompiledPatterns::new("file[1].txt").unwrap();
        assert!(patterns.matches("file[1].txt"));
        let patterns = CompiledPatterns::new("a+b").unwrap();
        assert!(patterns.matches("a+b"));
        let patterns = CompiledPatterns::new("(test)").unwrap();
        assert!(patterns.matches("(test)"));
    }

    #[test]
    fn compiled_patterns_mixed_exact_and_glob() {
        // Mix of exact matches (fast path) and globs (regex)
        let patterns = CompiledPatterns::new("node_modules|*.log|dist").unwrap();
        assert!(patterns.matches("node_modules")); // exact
        assert!(patterns.matches("dist")); // exact
        assert!(patterns.matches("debug.log")); // glob
        assert!(patterns.matches("error.log")); // glob
        assert!(!patterns.matches("main.rs"));
    }
}
