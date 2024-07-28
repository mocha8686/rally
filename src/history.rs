use std::{fs::create_dir_all, path::PathBuf};

use directories::BaseDirs;

pub fn get_history_path(name: &str) -> Option<PathBuf> {
    let Some(dirs) = BaseDirs::new() else {
        return None;
    };
    let dir = dirs.data_dir();
    let dir = dir.join(env!("CARGO_PKG_NAME"));

    create_dir_all(dir.clone()).ok();

    let dir = dir.join(format!("{name}_history"));
    Some(dir)
}
