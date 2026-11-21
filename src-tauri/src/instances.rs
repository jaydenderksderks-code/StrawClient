use serde::Serialize;
use std::{fs, path::{Path, PathBuf}};

#[derive(Serialize)]
pub struct InstanceInfo {
    pub name: String,
    pub directory: String,
}

fn valid_name(name: &str) -> bool {
    !name.trim().is_empty() && name.len() <= 48 &&
        name.chars().all(|c| c.is_ascii_alphanumeric() || c == ' ' || c == '-' || c == '_')
}

pub fn list(root: &Path) -> Result<Vec<InstanceInfo>, String> {
    let base = root.join("instances");
    fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for entry in fs::read_dir(&base).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            result.push(InstanceInfo { name: entry.file_name().to_string_lossy().into_owned(), directory: entry.path().to_string_lossy().into_owned() });
        }
    }
    result.sort_by(|a,b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(result)
}

pub fn create(root: &Path, name: &str) -> Result<InstanceInfo, String> {
    if !valid_name(name) { return Err("Instance names may contain letters, numbers, spaces, - and _ (max 48 characters).".into()); }
    let path: PathBuf = root.join("instances").join(name.trim());
    if path.exists() { return Err("An instance with that name already exists.".into()); }
    fs::create_dir_all(path.join("mods")).map_err(|e| e.to_string())?;
    fs::create_dir_all(path.join("resourcepacks")).map_err(|e| e.to_string())?;
    fs::create_dir_all(path.join("saves")).map_err(|e| e.to_string())?;
    Ok(InstanceInfo { name: name.trim().into(), directory: path.to_string_lossy().into_owned() })
}
