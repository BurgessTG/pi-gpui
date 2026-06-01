use std::fs;
use std::path::{Path, PathBuf};

use gpui_component::tree::TreeItem;

pub const DEFAULT_DIRECTORY_DEPTH: usize = 3;

pub fn build_directory_tree(root: &Path, max_depth: usize) -> Vec<TreeItem> {
    build_directory_tree_with_expanded_path(root, max_depth, root)
}

pub fn build_directory_tree_with_expanded_path(
    root: &Path,
    max_depth: usize,
    expanded_path: &Path,
) -> Vec<TreeItem> {
    vec![build_directory_item(root, 0, max_depth, expanded_path)]
}

fn build_directory_item(
    path: &Path,
    depth: usize,
    max_depth: usize,
    expanded_path: &Path,
) -> TreeItem {
    let children = if depth < max_depth {
        child_directories(path)
            .into_iter()
            .map(|child| build_directory_item(&child, depth + 1, max_depth, expanded_path))
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };

    TreeItem::new(path.to_string_lossy().to_string(), directory_label(path))
        .children(children)
        .expanded(depth == 0 || expanded_path.starts_with(path))
}

fn child_directories(path: &Path) -> Vec<PathBuf> {
    let mut directories = fs::read_dir(path)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter(|path| !is_hidden(path))
        .collect::<Vec<_>>();

    directories.sort_by_key(|path| directory_label(path).to_lowercase());
    directories
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.starts_with('.'))
}

fn directory_label(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| path.display().to_string())
}

#[cfg(test)]
mod tests {
    use super::build_directory_tree;
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn directory_tree_expands_selected_path_ancestors() -> Result<(), Box<dyn std::error::Error>> {
        let root = std::env::temp_dir().join(format!(
            "pi-picker-expanded-test-{}",
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
        ));
        let selected = root.join("alpha").join("nested");
        fs::create_dir_all(&selected)?;

        let tree = super::build_directory_tree_with_expanded_path(&root, 3, &selected);

        assert!(tree[0].is_expanded());
        assert!(tree[0].children[0].is_expanded());

        fs::remove_dir_all(root)?;
        Ok(())
    }

    #[test]
    fn directory_tree_contains_sorted_visible_directories() -> Result<(), Box<dyn std::error::Error>>
    {
        let root = std::env::temp_dir().join(format!(
            "pi-picker-test-{}",
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
        ));
        fs::create_dir_all(root.join("zeta"))?;
        fs::create_dir_all(root.join("alpha"))?;
        fs::create_dir_all(root.join(".hidden"))?;
        fs::write(root.join("file.txt"), "not a directory")?;

        let tree = build_directory_tree(&root, 1);
        let labels = tree[0]
            .children
            .iter()
            .map(|item| item.label.to_string())
            .collect::<Vec<_>>();

        assert_eq!(labels, vec!["alpha", "zeta"]);

        fs::remove_dir_all(root)?;
        Ok(())
    }
}
