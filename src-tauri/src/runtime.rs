use std::{env, path::PathBuf, process::Command};

#[derive(serde::Serialize)]
pub struct JavaRuntime {
    pub path: String,
    pub version: String,
    pub supported: bool,
}

fn inspect(path: PathBuf) -> Option<JavaRuntime> {
    let output = Command::new(&path).arg("-version").output().ok()?;
    if !output.status.success() { return None; }
    let text = format!("{}{}", String::from_utf8_lossy(&output.stderr), String::from_utf8_lossy(&output.stdout));
    let version = text.lines().next().unwrap_or("Java version unknown").trim().to_owned();
    let major = text.split('"').nth(1).and_then(|v| {
        let first = v.split('.').next()?;
        if first == "1" { v.split('.').nth(1)?.parse::<u32>().ok() } else { first.parse::<u32>().ok() }
    }).unwrap_or(0);
    Some(JavaRuntime { path: path.to_string_lossy().into_owned(), version, supported: major >= 8 })
}

pub fn detect_java() -> Vec<JavaRuntime> {
    let mut candidates = Vec::new();
    if let Ok(value) = env::var("JAVA_HOME") {
        candidates.push(PathBuf::from(value).join("bin").join(if cfg!(windows) { "java.exe" } else { "java" }));
    }
    if let Some(paths) = env::var_os("PATH") {
        for dir in env::split_paths(&paths) {
            candidates.push(dir.join(if cfg!(windows) { "java.exe" } else { "java" }));
        }
    }
    #[cfg(target_os = "linux")]
    candidates.extend([PathBuf::from("/usr/bin/java"), PathBuf::from("/usr/lib/jvm/default/bin/java")]);
    #[cfg(target_os = "macos")]
    candidates.push(PathBuf::from("/usr/bin/java"));
    let mut found = Vec::new();
    for path in candidates {
        if let Some(runtime) = inspect(path) {
            if !found.iter().any(|item: &JavaRuntime| item.path == runtime.path) { found.push(runtime); }
        }
    }
    found
}
