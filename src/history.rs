use std::{fs::create_dir_all, path::PathBuf};

use directories::ProjectDirs;

pub fn get_history_path(name: &str) -> Option<PathBuf> {
    let dirs = ProjectDirs::from("", "", env!("CARGO_PKG_NAME"))?;
    let dir = dirs.data_dir();
    create_dir_all(dir).ok();
    let dir = dir.join(format!("{name}_history"));
    Some(dir)
}
