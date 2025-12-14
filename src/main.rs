use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn main() -> ExitCode {
    let raw_arg_path = std::env::args_os().nth(1);

    let (root_path, is_current_dir) = match raw_arg_path.as_ref() {
        None => (PathBuf::from("."), true),
        Some(raw) => (PathBuf::from(raw), Path::new(raw) == Path::new(".")),
    };

    let metadata = match std::fs::symlink_metadata(&root_path) {
        Ok(metadata) => metadata,
        Err(err) => {
            eprintln!("rtree: {}: {}", root_path.display(), err);
            return ExitCode::from(1);
        }
    };

    if !metadata.is_dir() {
        let file_name = root_path
            .file_name()
            .map(|name| name.to_string_lossy())
            .unwrap_or_else(|| root_path.as_os_str().to_string_lossy());
        println!("{file_name}");
        return ExitCode::SUCCESS;
    }

    println!("{}", rtree::root_display_name(&root_path, is_current_dir));
    ExitCode::SUCCESS
}
