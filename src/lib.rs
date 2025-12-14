use std::path::Path;

pub fn root_display_name(root_path: &Path, is_current_dir: bool) -> String {
    if is_current_dir {
        return ".".to_owned();
    }

    root_path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| root_path.as_os_str().to_string_lossy().into_owned())
}

