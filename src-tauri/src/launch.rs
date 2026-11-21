use serde_json::Value;
use std::{path::Path, process::{Command, Stdio}};

fn allowed(arg: &Value) -> bool {
    let Some(rules) = arg.get("rules").and_then(Value::as_array) else { return true; };
    let mut enabled = false;
    for rule in rules {
        let os = rule.get("os");
        let matches = os.and_then(|v| v.get("name")).and_then(Value::as_str)
            .map(|v| v == if cfg!(target_os = "windows") { "windows" } else if cfg!(target_os = "macos") { "osx" } else { "linux" }).unwrap_or(true);
        if matches { enabled = rule.get("action").and_then(Value::as_str) == Some("allow"); }
    }
    enabled
}

fn expand(value: &str, vars: &[(String, String)]) -> String {
    vars.iter().fold(value.to_owned(), |out, (key, val)| out.replace(&format!("${{{key}}}"), val))
}

fn add_argument(output: &mut Vec<String>, item: &Value, vars: &[(String, String)]) {
    if item.is_string() {
        output.push(expand(item.as_str().unwrap_or_default(), vars));
    } else if item.is_object() && allowed(item) {
        if let Some(values) = item.get("value") {
            if let Some(s) = values.as_str() { output.push(expand(s, vars)); }
            if let Some(list) = values.as_array() {
                for value in list { if let Some(s) = value.as_str() { output.push(expand(s, vars)); } }
            }
        }
    }
}

pub fn launch(root: &Path, version: &str, username: &str, java: &str) -> Result<u32, String> {
    if username.trim().is_empty() || username.len() > 16 || !username.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err("Offline username must be 1–16 letters, numbers or underscores.".into());
    }
    let version_dir = root.join("versions").join(version);
    let metadata_path = version_dir.join(format!("{version}.json"));
    let metadata: Value = serde_json::from_slice(&std::fs::read(&metadata_path).map_err(|e| format!("Version is not installed: {e}"))?)
        .map_err(|e| format!("Invalid version metadata: {e}"))?;
    let game_dir = root.join("instances").join("Strawberry");
    std::fs::create_dir_all(&game_dir).map_err(|e| e.to_string())?;
    let assets = root.join("assets");
    let natives = version_dir.join("natives");
    let separator = if cfg!(windows) { ";" } else { ":" };
    let mut classpath = Vec::new();
    if let Some(libraries) = metadata.get("libraries").and_then(Value::as_array) {
        for library in libraries {
            if let Some(path) = library.pointer("/downloads/artifact/path").and_then(Value::as_str) {
                let candidate = root.join("libraries").join(path);
                if candidate.exists() { classpath.push(candidate.to_string_lossy().into_owned()); }
            }
        }
    }
    let client_jar = version_dir.join(format!("{version}.jar"));
    if !client_jar.exists() { return Err("Minecraft client JAR is missing. Install this version first.".into()); }
    classpath.push(client_jar.to_string_lossy().into_owned());
    let cp = classpath.join(separator);
    let asset_index = metadata.pointer("/assetIndex/id").and_then(Value::as_str).unwrap_or(version);
    let uuid = format!("offline-{}", username.to_lowercase());
    let vars = vec![
        ("auth_player_name".into(), username.into()),
        ("version_name".into(), version.into()),
        ("game_directory".into(), game_dir.to_string_lossy().into_owned()),
        ("assets_root".into(), assets.to_string_lossy().into_owned()),
        ("assets_index_name".into(), asset_index.into()),
        ("auth_uuid".into(), uuid),
        ("auth_access_token".into(), "0".into()),
        ("user_type".into(), "legacy".into()),
        ("version_type".into(), "release".into()),
        ("natives_directory".into(), natives.to_string_lossy().into_owned()),
        ("launcher_name".into(), "StrawClient".into()),
        ("launcher_version".into(), env!("CARGO_PKG_VERSION").into()),
        ("classpath".into(), cp),
        ("library_directory".into(), root.join("libraries").to_string_lossy().into_owned()),
        ("classpath_separator".into(), separator.into()),
    ];
    let main_class = metadata.get("mainClass").and_then(Value::as_str).ok_or("Version metadata has no main class")?;
    let mut args = Vec::new();
    if let Some(jvm) = metadata.pointer("/arguments/jvm").and_then(Value::as_array) {
        for item in jvm { add_argument(&mut args, item, &vars); }
    } else {
        args.push(format!("-Djava.library.path={}", natives.to_string_lossy()));
        args.push("-cp".into());
        args.push(vars.iter().find(|v| v.0 == "classpath").unwrap().1.clone());
    }
    args.push(main_class.into());
    if let Some(game) = metadata.pointer("/arguments/game").and_then(Value::as_array) {
        for item in game { add_argument(&mut args, item, &vars); }
    } else if let Some(legacy) = metadata.get("minecraftArguments").and_then(Value::as_str) {
        args.extend(legacy.split_whitespace().map(|v| expand(v, &vars)));
    } else {
        return Err("No supported Minecraft game arguments were found.".into());
    }
    let child = Command::new(java).args(args).current_dir(&game_dir)
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
        .spawn().map_err(|e| format!("Could not start Java: {e}"))?;
    Ok(child.id())
}
