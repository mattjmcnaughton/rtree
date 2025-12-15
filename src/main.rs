use clap::Parser;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

mod cli;

#[tokio::main]
async fn main() -> ExitCode {
    let args = cli::Cli::parse();
    let root_path = args.path.unwrap_or_else(|| PathBuf::from("."));
    let is_current_dir = root_path == Path::new(".");

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

    let fs = rtree::fs::RealFileSystem;
    let tree = rtree::core::walk::walk_dir(&fs, &root_path).await;

    let mut stdout = std::io::stdout().lock();
    if let Err(err) = (|| -> std::io::Result<()> {
        write!(
            &mut stdout,
            "{}",
            rtree::root_display_name(&root_path, is_current_dir)
        )?;
        if let Some(error) = tree.error.as_ref() {
            write!(&mut stdout, " [error: {error}]")?;
        }
        writeln!(&mut stdout)?;
        rtree::core::render::write_children(&mut stdout, &tree.children)?;
        Ok(())
    })() {
        eprintln!("rtree: stdout: {err}");
        return ExitCode::from(1);
    }

    ExitCode::SUCCESS
}
