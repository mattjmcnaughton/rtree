use super::EntryKind;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TreeNode {
    pub name: String,
    pub kind: EntryKind,
    pub error: Option<String>,
    pub children: Vec<TreeNode>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirTree {
    pub error: Option<String>,
    pub children: Vec<TreeNode>,
}
