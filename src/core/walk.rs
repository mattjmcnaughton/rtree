use std::path::Path;

use crate::fs::FileSystem;
use crate::models::{DirTree, EntryKind, TreeNode};

pub async fn walk_dir<F: FileSystem>(fs: &F, dir: &Path) -> DirTree {
    let entries = match fs.read_dir(dir).await {
        Ok(entries) => entries,
        Err(err) => {
            return DirTree {
                error: Some(err.to_string()),
                children: Vec::new(),
            };
        }
    };

    let mut entries_with_rendered: Vec<(String, _)> = entries
        .into_iter()
        .map(|entry| (rendered_name(&entry.name, entry.kind), entry))
        .collect();
    entries_with_rendered.sort_by(|(a, _), (b, _)| a.cmp(b));

    let mut children = Vec::with_capacity(entries_with_rendered.len());
    for (rendered, entry) in entries_with_rendered {
        let mut node = TreeNode {
            name: rendered,
            kind: entry.kind,
            error: None,
            children: Vec::new(),
        };

        if entry.kind == EntryKind::Directory {
            let subtree = Box::pin(walk_dir(fs, &entry.path)).await;
            node.error = subtree.error;
            node.children = subtree.children;
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

        let tree = walk_dir(&fs, Path::new("/root")).await;
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

        let tree = walk_dir(&fs, Path::new("/root")).await;
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

        let tree = walk_dir(&fs, Path::new("/root")).await;
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
}
