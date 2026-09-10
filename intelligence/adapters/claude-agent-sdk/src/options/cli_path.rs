use crate::error::ClaudeAgentError;
use crate::error::Result;
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;

pub(crate) fn find_cli(explicit: Option<&PathBuf>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        if path.exists() {
            return Ok(path.clone());
        }
        if is_bare_command(path)
            && let Some(resolved) = find_on_path(path)
        {
            return Ok(resolved);
        }
        return Err(ClaudeAgentError::CliNotFoundAt { path: path.clone() });
    }

    if let Some(path) = find_on_path(Path::new("claude")) {
        return Ok(path);
    }

    let home = env::var_os("HOME").map(PathBuf::from);
    let candidates = home
        .into_iter()
        .flat_map(|home| {
            vec![
                home.join(".npm-global/bin/claude"),
                home.join(".local/bin/claude"),
                home.join("node_modules/.bin/claude"),
                home.join(".yarn/bin/claude"),
                home.join(".claude/local/claude"),
            ]
        })
        .chain([PathBuf::from("/usr/local/bin/claude")]);

    for candidate in candidates {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    Err(ClaudeAgentError::CliNotFound)
}

fn is_bare_command(path: &Path) -> bool {
    let mut components = path.components();
    matches!(components.next(), Some(std::path::Component::Normal(_)))
        && components.next().is_none()
}

fn find_on_path(binary: &Path) -> Option<PathBuf> {
    let path = env::var_os("PATH")?;
    env::split_paths(&path)
        .map(|dir| dir.join(binary))
        .find(|path| path.exists())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{ffi::OsString, sync::Mutex};
    use tempfile::tempdir;
    static PATH_ENV_LOCK: Mutex<()> = Mutex::new(());

    struct PathEnvGuard {
        previous: Option<OsString>,
    }

    impl PathEnvGuard {
        fn prepend(path: &std::path::Path) -> Self {
            let previous = std::env::var_os("PATH");
            let paths = std::iter::once(path.to_path_buf())
                .chain(
                    previous
                        .as_ref()
                        .into_iter()
                        .flat_map(|value| std::env::split_paths(value)),
                )
                .collect::<Vec<_>>();
            let joined = std::env::join_paths(paths).unwrap();
            // SAFETY: This test serializes PATH mutations with PATH_ENV_LOCK and
            // restores the previous value in Drop before releasing the lock.
            unsafe {
                std::env::set_var("PATH", joined);
            }
            Self { previous }
        }
    }

    impl Drop for PathEnvGuard {
        fn drop(&mut self) {
            // SAFETY: Protected by PATH_ENV_LOCK for the full guard lifetime.
            unsafe {
                match &self.previous {
                    Some(value) => std::env::set_var("PATH", value),
                    None => std::env::remove_var("PATH"),
                }
            }
        }
    }

    #[test]
    fn explicit_bare_cli_path_resolves_through_path() {
        let _lock = PATH_ENV_LOCK.lock().unwrap();
        let dir = tempdir().unwrap();
        let cli = dir.path().join("claude");
        fs::write(&cli, "#!/bin/sh\n").unwrap();
        let _path_guard = PathEnvGuard::prepend(dir.path());

        let requested = PathBuf::from("claude");
        assert_eq!(find_cli(Some(&requested)).unwrap(), cli);
    }
}
