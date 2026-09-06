use std::collections::BTreeMap;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

fn main() {
    if env::args_os().nth(1).as_deref() == Some(OsStr::new("--retained-pipe")) {
        std::thread::sleep(Duration::from_secs(30));
        return;
    }
    if let Err(message) = run() {
        eprintln!("fixture provider contract refusal: {message}");
        std::process::exit(64);
    }
}

fn run() -> Result<(), String> {
    let args: Vec<OsString> = env::args_os().skip(1).collect();
    if args.len() != 9 {
        return Err(format!("expected 9 arguments, received {}", args.len()));
    }
    expect_arg(&args, 0, "-c")?;
    expect_arg(&args, 2, "--cargo-path")?;
    expect_arg(&args, 4, "--rustc-path")?;
    expect_arg(&args, 6, "--cargo-options=--locked")?;
    expect_arg(&args, 7, "buckify")?;
    expect_arg(&args, 8, "--stdout")?;

    let config = absolute_path(&args[1], "config")?;
    let cargo = absolute_path(&args[3], "cargo")?;
    let rustc = absolute_path(&args[5], "rustc")?;
    if !config.ends_with("reindeer.toml") || !config.is_file() {
        return Err("config argument is not an existing reindeer.toml".to_owned());
    }
    if !cargo.is_file() || !rustc.is_file() {
        return Err("cargo and rustc arguments must be files".to_owned());
    }
    let candidate_root = config
        .parent()
        .ok_or_else(|| "config has no parent".to_owned())?;
    if env::current_dir().map_err(|error| error.to_string())? != candidate_root {
        return Err("working directory is not the candidate root".to_owned());
    }

    let actual_env: BTreeMap<OsString, OsString> = env::vars_os().collect();
    let target_dir = actual_env
        .get(OsStr::new("CARGO_TARGET_DIR"))
        .ok_or_else(|| "CARGO_TARGET_DIR is missing".to_owned())?;
    let target_dir = absolute_path(target_dir, "target directory")?;
    let target_name = target_dir.file_name().and_then(OsStr::to_str);
    if !matches!(target_name, Some("run-one" | "run-two")) || !target_dir.is_dir() {
        return Err("CARGO_TARGET_DIR is not a created independent run directory".to_owned());
    }
    let mut expected_env = BTreeMap::new();
    expected_env.insert(
        OsString::from("CARGO_HOME"),
        candidate_root.join("third-party/.cargo").into_os_string(),
    );
    expected_env.insert(OsString::from("CARGO_NET_OFFLINE"), OsString::from("true"));
    expected_env.insert(
        OsString::from("CARGO_TARGET_DIR"),
        target_dir.as_os_str().to_owned(),
    );
    expected_env.insert(OsString::from("LANG"), OsString::from("C"));
    expected_env.insert(OsString::from("LC_ALL"), OsString::from("C"));
    expected_env.insert(OsString::from("TZ"), OsString::from("UTC"));
    if actual_env != expected_env {
        return Err(format!(
            "environment is not the exact allowlist: expected {expected_env:?}, actual {actual_env:?}"
        ));
    }
    fs::write(target_dir.join("provider-ran"), b"target-only mutation\n")
        .map_err(|error| error.to_string())?;

    let executable = env::current_exe().map_err(|error| error.to_string())?;
    let mode = executable
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| "fixture executable name is not UTF-8".to_owned())?;
    let first_run = candidate_root.ends_with("candidate-one");

    if mode.contains("partial-failure") {
        io::stdout()
            .write_all(b"partial")
            .map_err(|error| error.to_string())?;
        std::process::exit(7);
    }
    if mode.contains("exit-diagnostic") {
        io::stderr()
            .write_all(b"provider refused\n")
            .map_err(|error| error.to_string())?;
        std::process::exit(9);
    }
    if mode.contains("stderr-success") {
        io::stdout()
            .write_all(b"generated\n")
            .map_err(|error| error.to_string())?;
        io::stderr()
            .write_all(b"unexpected diagnostic\n")
            .map_err(|error| error.to_string())?;
        return Ok(());
    }
    if mode.contains("empty-success") {
        return Ok(());
    }
    if mode.contains("timeout") {
        std::thread::sleep(Duration::from_secs(2));
    }
    if mode.contains("descendant") {
        let mut descendant = std::process::Command::new(env::current_exe().unwrap())
            .arg("--retained-pipe")
            .spawn()
            .map_err(|error| error.to_string())?;
        fs::write(
            target_dir.join("descendant-pid"),
            descendant.id().to_string(),
        )
        .map_err(|error| error.to_string())?;
        if mode.contains("wait-descendant") {
            descendant.wait().map_err(|error| error.to_string())?;
        }
    }
    if mode.contains("stdout-limit") {
        io::stdout()
            .write_all(&vec![b'x'; 4096])
            .map_err(|error| error.to_string())?;
        return Ok(());
    }
    if mode.contains("stderr-limit") {
        io::stdout()
            .write_all(b"generated\n")
            .map_err(|error| error.to_string())?;
        io::stderr()
            .write_all(&vec![b'e'; 4096])
            .map_err(|error| error.to_string())?;
        return Ok(());
    }
    if mode.contains("mutate-cargo-lock") {
        fs::write(candidate_root.join("Cargo.lock"), b"mutated\n")
            .map_err(|error| error.to_string())?;
    }
    if mode.contains("mutate-third-party-buck") {
        fs::write(candidate_root.join("third-party/BUCK"), b"mutated\n")
            .map_err(|error| error.to_string())?;
    }
    if mode.contains("mutate-config") {
        fs::write(candidate_root.join("reindeer.toml"), b"mutated\n")
            .map_err(|error| error.to_string())?;
    }
    if mode.contains("mutate-manifest") {
        fs::write(candidate_root.join("Cargo.toml"), b"mutated\n")
            .map_err(|error| error.to_string())?;
    }
    if mode.contains("mutate-source") {
        fs::write(candidate_root.join("src/lib.rs"), b"mutated\n")
            .map_err(|error| error.to_string())?;
    }
    if mode.contains("add-input") {
        fs::write(candidate_root.join("new-input"), b"added\n")
            .map_err(|error| error.to_string())?;
    }
    if mode.contains("mutate-second-semantic") && first_run {
        let second = sibling_root(candidate_root, "candidate-two")?;
        fs::write(second.join("src/lib.rs"), b"cross-root mutation\n")
            .map_err(|error| error.to_string())?;
    }
    if mode.contains("mutate-second-cache") && first_run {
        let second = sibling_root(candidate_root, "candidate-two")?;
        fs::write(
            second.join("third-party/.cargo/seed"),
            b"cross-cache mutation\n",
        )
        .map_err(|error| error.to_string())?;
    }
    if mode.contains("mutate-own-cache") {
        fs::write(
            candidate_root.join("third-party/.cargo/runtime-state"),
            b"allowed cache state\n",
        )
        .map_err(|error| error.to_string())?;
    }
    if mode.contains("mutate-first-cache-from-second") && !first_run {
        let first = sibling_root(candidate_root, "candidate-one")?;
        fs::write(
            first.join("third-party/.cargo/seed"),
            b"reverse cross-cache mutation\n",
        )
        .map_err(|error| error.to_string())?;
    }
    if mode.contains("nondeterministic") {
        let bytes: &[u8] = if first_run { b"first\n" } else { b"second\n" };
        io::stdout()
            .write_all(bytes)
            .map_err(|error| error.to_string())?;
        return Ok(());
    }
    io::stdout()
        .write_all(b"generated\n")
        .map_err(|error| error.to_string())?;
    Ok(())
}

fn sibling_root(candidate_root: &Path, name: &str) -> Result<PathBuf, String> {
    candidate_root
        .parent()
        .map(|parent| parent.join(name))
        .ok_or_else(|| "candidate root has no parent".to_owned())
}

fn expect_arg(args: &[OsString], index: usize, expected: &str) -> Result<(), String> {
    if args.get(index).is_some_and(|actual| actual == expected) {
        Ok(())
    } else {
        Err(format!("argument {index} is not {expected:?}"))
    }
}

fn absolute_path(value: &OsStr, label: &str) -> Result<PathBuf, String> {
    let path = Path::new(value);
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Err(format!("{label} is not absolute"))
    }
}
