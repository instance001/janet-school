use std::path::{Path, PathBuf};

const APP_ROOT_ENV: &str = "JANET_SCHOOL_BASE_PATH";
pub const APP_LAYOUT_DIRS: &[&str] = &[
    "assets",
    "bridge",
    "compare_exports",
    "config",
    "data",
    "data/aggregated",
    "data/sessions",
    "models",
    "runtime",
    "runtime/windows",
    "web",
];

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
    std::fs::create_dir_all(root)?;
    for relative in APP_LAYOUT_DIRS {
        std::fs::create_dir_all(app_layout_path(root, relative))?;
    }
    Ok(())
}

pub fn app_layout_path(root: &Path, relative: &str) -> PathBuf {
    let mut path = root.to_path_buf();
    for component in relative.split('/') {
        if !component.is_empty() {
            path.push(component);
        }
    }
    path
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

#[cfg(test)]
mod tests {
    use std::{
        fs,
        path::PathBuf,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{APP_LAYOUT_DIRS, app_layout_path, ensure_app_layout};

    fn temp_dir(label: &str) -> PathBuf {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("janet-school-app-paths-{label}-{unique}"))
    }

    #[test]
    fn ensure_app_layout_bootstraps_binary_first_run_layout() {
        let root = temp_dir("first-run-layout");

        ensure_app_layout(&root).unwrap();

        assert!(root.is_dir());
        for relative in APP_LAYOUT_DIRS {
            let path = app_layout_path(&root, relative);
            assert!(
                path.is_dir(),
                "expected first-run folder {} to exist",
                path.display()
            );
        }

        fs::remove_dir_all(root).unwrap();
    }
}
