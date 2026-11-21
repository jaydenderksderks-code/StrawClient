use futures_util::stream::{self, StreamExt};
use serde_json::Value;
use sha1::{Digest, Sha1};
use std::{path::{Path, PathBuf}, sync::Arc};
use tauri::{AppHandle, Emitter, Manager};

const MANIFEST_URL: &str = "https://piston-meta.mojang.com/mc/game/version_manifest_v2.json";
const ASSET_BASE: &str = "https://resources.download.minecraft.net";

fn platform_name() -> &'static str {
    if cfg!(target_os = "windows") { "windows" }
    else if cfg!(target_os = "macos") { "osx" }
    else { "linux" }
}

fn safe_join(root: &Path, relative: &str) -> Result<PathBuf, String> {
    let rel = Path::new(relative);
    if rel.is_absolute() || rel.components().any(|c| !matches!(c, std::path::Component::Normal(_))) {
        return Err(format!("Unsafe download path: {relative}"));
    }
    Ok(root.join(rel))
}

async fn download_file(client: &reqwest::Client, url: &str, target: &Path, expected_sha1: Option<&str>) -> Result<(), String> {
    if let Some(parent) = target.parent() {
        tokio::fs::create_dir_all(parent).await.map_err(|e| e.to_string())?;
    }
    if target.exists() {
        if let Some(expected) = expected_sha1 {
            let bytes = tokio::fs::read(target).await.map_err(|e| e.to_string())?;
            let actual = hex::encode(Sha1::digest(&bytes));
            if actual.eq_ignore_ascii_case(expected) { return Ok(()); }
        } else { return Ok(()); }
    }
    let response = client.get(url).send().await.map_err(|e| format!("Download failed ({url}): {e}"))?;
    if !response.status().is_success() {
        return Err(format!("HTTP {} while downloading {url}", response.status()));
    }
    let bytes = response.bytes().await.map_err(|e| e.to_string())?;
    if let Some(expected) = expected_sha1 {
        let actual = hex::encode(Sha1::digest(&bytes));
        if !actual.eq_ignore_ascii_case(expected) {
            return Err(format!("SHA-1 mismatch for {url}: expected {expected}, got {actual}"));
        }
    }
    let temp = target.with_extension("strawclient-tmp");
    tokio::fs::write(&temp, &bytes).await.map_err(|e| e.to_string())?;
    tokio::fs::rename(&temp, target).await.map_err(|e| e.to_string())?;
    Ok(())
}

fn library_allowed(library: &Value) -> bool {
    let Some(rules) = library.get("rules").and_then(Value::as_array) else { return true; };
    let mut allowed = false;
    for rule in rules {
        let os_matches = rule.get("os").and_then(|v| v.get("name")).and_then(Value::as_str)
            .map(|name| name == platform_name()).unwrap_or(true);
        if os_matches {
            allowed = rule.get("action").and_then(Value::as_str) == Some("allow");
        }
    }
    allowed
}

#[tauri::command]
async fn platform_info() -> String {
    format!("NATIVE BACKEND ONLINE · {}", std::env::consts::OS.to_uppercase())
}

#[tauri::command]
async fn install_version(app: AppHandle, version_id: String) -> Result<String, String> {
    if version_id.is_empty() || version_id.len() > 64 || !version_id.chars().all(|c| c.is_ascii_alphanumeric() || "._-".contains(c)) {
        return Err("Invalid Minecraft version identifier".into());
    }
    let client = reqwest::Client::builder().user_agent("StrawClient/0.1.0").build().map_err(|e| e.to_string())?;
    let root = app.path().app_data_dir().map_err(|e| e.to_string())?.join("minecraft");
    let versions_dir = root.join("versions").join(&version_id);
    tokio::fs::create_dir_all(&versions_dir).await.map_err(|e| e.to_string())?;

    app.emit("install-progress", format!("Fetching Mojang version manifest for {version_id}...")).ok();
    let manifest: Value = client.get(MANIFEST_URL).send().await.map_err(|e| e.to_string())?
        .error_for_status().map_err(|e| e.to_string())?.json().await.map_err(|e| e.to_string())?;
    let entry = manifest.get("versions").and_then(Value::as_array)
        .and_then(|list| list.iter().find(|v| v.get("id").and_then(Value::as_str) == Some(&version_id)))
        .ok_or_else(|| format!("Minecraft version {version_id} is not in Mojang's manifest"))?;
    let metadata_url = entry.get("url").and_then(Value::as_str).ok_or("Version metadata URL missing")?;
    let metadata: Value = client.get(metadata_url).send().await.map_err(|e| e.to_string())?
        .error_for_status().map_err(|e| e.to_string())?.json().await.map_err(|e| e.to_string())?;
    let metadata_path = versions_dir.join(format!("{version_id}.json"));
    tokio::fs::write(&metadata_path, serde_json::to_vec_pretty(&metadata).map_err(|e| e.to_string())?)
        .await.map_err(|e| e.to_string())?;

    app.emit("install-progress", "Downloading Minecraft client...").ok();
    let client_info = metadata.get("downloads").and_then(|v| v.get("client")).ok_or("This version has no downloadable client artifact")?;
    let client_url = client_info.get("url").and_then(Value::as_str).ok_or("Client download URL missing")?;
    let client_sha = client_info.get("sha1").and_then(Value::as_str);
    let client_path = versions_dir.join(format!("{version_id}.jar"));
    download_file(&client, client_url, &client_path, client_sha).await?;

    let libraries_root = root.join("libraries");
    let mut library_jobs = Vec::new();
    if let Some(libraries) = metadata.get("libraries").and_then(Value::as_array) {
        for library in libraries {
            if !library_allowed(library) { continue; }
            if let Some(artifact) = library.get("downloads").and_then(|v| v.get("artifact")) {
                if let (Some(url), Some(path)) = (artifact.get("url").and_then(Value::as_str), artifact.get("path").and_then(Value::as_str)) {
                    let target = safe_join(&libraries_root, path)?;
                    let sha = artifact.get("sha1").and_then(Value::as_str).map(str::to_owned);
                    library_jobs.push((url.to_owned(), target, sha));
                }
            }
        }
    }
    let total = library_jobs.len();
    app.emit("install-progress", format!("Preparing {total} libraries...")).ok();
    let http = Arc::new(client.clone());
    let results = stream::iter(library_jobs.into_iter().map(|(url, path, sha)| {
        let http = Arc::clone(&http);
        async move { download_file(&http, &url, &path, sha.as_deref()).await }
    })).buffer_unordered(8).collect::<Vec<_>>().await;
    for result in results { result?; }

    if let Some(asset_index) = metadata.get("assetIndex") {
        let index_url = asset_index.get("url").and_then(Value::as_str).ok_or("Asset index URL missing")?;
        let index_id = asset_index.get("id").and_then(Value::as_str).ok_or("Asset index ID missing")?;
        let index_path = root.join("assets").join("indexes").join(format!("{index_id}.json"));
        app.emit("install-progress", "Downloading asset index...").ok();
        download_file(&client, index_url, &index_path, asset_index.get("sha1").and_then(Value::as_str)).await?;
        let index: Value = tokio::fs::read(&index_path).await.map_err(|e| e.to_string())
            .and_then(|b| serde_json::from_slice(&b).map_err(|e| e.to_string()))?;
        let objects = index.get("objects").and_then(Value::as_object).ok_or("Asset index has no objects")?;
        let asset_root = root.join("assets").join("objects");
        let jobs: Vec<(String, PathBuf, String)> = objects.values().filter_map(|object| {
            let hash = object.get("hash")?.as_str()?.to_owned();
            if hash.len() != 40 || !hash.chars().all(|c| c.is_ascii_hexdigit()) { return None; }
            let target = asset_root.join(&hash[..2]).join(&hash);
            Some((format!("{ASSET_BASE}/{}/{}", &hash[..2], hash), target, hash))
        }).collect();
        let count = jobs.len();
        app.emit("install-progress", format!("Verifying and downloading {count} assets (8 at a time)...")).ok();
        let http = Arc::new(client.clone());
        let results = stream::iter(jobs.into_iter().map(|(url, path, sha)| {
            let http = Arc::clone(&http);
            async move { download_file(&http, &url, &path, Some(&sha)).await }
        })).buffer_unordered(8).collect::<Vec<_>>().await;
        for result in results { result?; }
    }

    app.emit("install-progress", format!("Minecraft {version_id} files installed. Java runtime and launch profile are the next setup stage.")).ok();
    Ok(root.to_string_lossy().into_owned())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![platform_info, install_version])
        .run(tauri::generate_context!())
        .expect("error while running StrawClient");
}
