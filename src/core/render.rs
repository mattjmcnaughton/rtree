use std::io::{self, Write};

use crate::models::TreeNode;

pub fn write_children<W: Write>(writer: &mut W, children: &[TreeNode]) -> io::Result<()> {
    write_children_inner(writer, children, &[])
}

fn write_children_inner<W: Write>(
    writer: &mut W,
    children: &[TreeNode],
    ancestor_has_more: &[bool],
) -> io::Result<()> {
    for (index, node) in children.iter().enumerate() {
        let is_last = index + 1 == children.len();

        for &has_more in ancestor_has_more {
            if has_more {
                writer.write_all(b"|   ")?;
            } else {
                writer.write_all(b"    ")?;
            }
        }

        if is_last {
            writer.write_all(b"`-- ")?;
        } else {
            writer.write_all(b"|-- ")?;
        }

        writer.write_all(node.name.as_bytes())?;

        if let Some(error) = node.error.as_ref() {
            write!(writer, " [error: {error}]")?;
        }

        writer.write_all(b"\n")?;

        if !node.children.is_empty() {
            let mut next_ancestor_has_more = ancestor_has_more.to_vec();
            next_ancestor_has_more.push(!is_last);
            write_children_inner(writer, &node.children, &next_ancestor_has_more)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{EntryKind, TreeNode};

    #[test]
    fn renders_scaffold_and_errors() {
        let children = vec![
            TreeNode {
                name: "a".to_owned(),
                kind: EntryKind::File,
                error: None,
                children: vec![],
            },
            TreeNode {
                name: "b/".to_owned(),
                kind: EntryKind::Directory,
                error: Some("Permission denied".to_owned()),
                children: vec![],
            },
            TreeNode {
                name: "c/".to_owned(),
                kind: EntryKind::Directory,
                error: None,
                children: vec![TreeNode {
                    name: "d".to_owned(),
                    kind: EntryKind::File,
                    error: None,
                    children: vec![],
                }],
            },
        ];

        let mut out = Vec::new();
        write_children(&mut out, &children).unwrap();
        let out = String::from_utf8(out).unwrap();

        assert_eq!(
            out,
            concat!(
                "|-- a\n",
                "|-- b/ [error: Permission denied]\n",
                "`-- c/\n",
                "    `-- d\n",
            )
        );
    }
}
