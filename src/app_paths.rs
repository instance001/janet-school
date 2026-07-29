use std::path::{Path, PathBuf};

const APP_ROOT_ENV: &str = "JANET_SCHOOL_BASE_PATH";

pub fn app_root() -> PathBuf {
    if let Some(path) = std::env::var_os(APP_ROOT_ENV)
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
    {
        return path;
    }

    let mut candidates = Vec::new();
    if let Ok(exe_path) = std::env::current_exe()
        && let Some(parent) = exe_path.parent()
    {
        candidates.push(parent.to_path_buf());
    }
    if let Ok(current_dir) = std::env::current_dir() {
        candidates.push(current_dir);
    }

    for candidate in &candidates {
        if let Some(root) = find_source_root(candidate) {
            return root;
        }
    }

    candidates
        .into_iter()
        .next()
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn resolve(root: &Path, value: impl AsRef<Path>) -> PathBuf {
    let value = value.as_ref();
    if value.is_absolute() {
        value.to_path_buf()
    } else {
        root.join(value)
    }
}

pub fn ensure_app_layout(root: &Path) -> std::io::Result<()> {
    for relative in [
        "config",
        "data",
        "data/sessions",
        "data/aggregated",
        "compare_exports",
        "models",
        "runtime",
        "bridge",
        "web",
    ] {
        std::fs::create_dir_all(root.join(relative))?;
    }
    Ok(())
}

fn find_source_root(start: &Path) -> Option<PathBuf> {
    for ancestor in start.ancestors() {
        if is_source_root(ancestor) {
            return Some(ancestor.to_path_buf());
        }
    }
    None
}

fn is_source_root(path: &Path) -> bool {
    path.join("Cargo.toml").exists()
        && path.join("src").join("main.rs").exists()
        && path.join("config").join("app_config.json").exists()
        && path.join("web").join("index.html").exists()
}
