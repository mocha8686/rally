use std::{fs::create_dir_all, path::PathBuf};

use directories::ProjectDirs;

pub fn get_history_path(name: &str) -> Option<PathBuf> {
    let Some(dirs) = ProjectDirs::from("", "", "rally") else {
        return None;
    };
    let dir = dirs.data_dir();

    create_dir_all(&dir).ok();

    let dir = dir.join(format!("{name}_history"));
    Some(dir)
}
