use std::collections::BTreeMap;
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

const TOOLCHAIN: &str = "1.98.0";

fn main() {
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

    let actual_env: BTreeMap<OsString, OsString> = env::vars_os().collect();
    let mut expected_env = BTreeMap::new();
    expected_env.insert(
        OsString::from("CARGO_HOME"),
        candidate_root.join("third-party/.cargo").into_os_string(),
    );
    expected_env.insert(
        OsString::from("CARGO_NET_OFFLINE"),
        OsString::from("true"),
    );
    let target_dir = actual_env
        .get(OsStr::new("CARGO_TARGET_DIR"))
        .ok_or_else(|| "CARGO_TARGET_DIR is missing".to_owned())?;
    let target_dir = absolute_path(target_dir, "target directory")?;
    let target_name = target_dir.file_name().and_then(OsStr::to_str);
    if !matches!(target_name, Some("run-one" | "run-two")) {
        return Err("CARGO_TARGET_DIR does not name an independent run".to_owned());
    }
    expected_env.insert(
        OsString::from("CARGO_TARGET_DIR"),
        target_dir.as_os_str().to_owned(),
    );
    expected_env.insert(OsString::from("LANG"), OsString::from("C"));
    expected_env.insert(OsString::from("LC_ALL"), OsString::from("C"));
    expected_env.insert(
        OsString::from("RUSTUP_TOOLCHAIN"),
        OsString::from(TOOLCHAIN),
    );
    expected_env.insert(OsString::from("TZ"), OsString::from("UTC"));
    if actual_env != expected_env {
        return Err(format!(
            "environment is not the exact allowlist: expected {expected_env:?}, actual {actual_env:?}"
        ));
    }

    let executable = env::current_exe().map_err(|error| error.to_string())?;
    let mode = executable
        .file_name()
        .and_then(OsStr::to_str)
        .ok_or_else(|| "fixture executable name is not UTF-8".to_owned())?;
    if mode.contains("partial-failure") {
        io::stdout()
            .write_all(b"partial")
            .map_err(|error| error.to_string())?;
        std::process::exit(7);
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
    if mode.contains("mutate-cargo-lock") {
        fs::write(candidate_root.join("Cargo.lock"), b"mutated\n")
            .map_err(|error| error.to_string())?;
    }
    if mode.contains("mutate-third-party-buck") {
        fs::write(candidate_root.join("third-party/BUCK"), b"mutated\n")
            .map_err(|error| error.to_string())?;
    }
    if mode.contains("nondeterministic") {
        io::stdout()
            .write_all(target_name.unwrap_or_default().as_bytes())
            .map_err(|error| error.to_string())?;
        return Ok(());
    }
    io::stdout()
        .write_all(b"generated\n")
        .map_err(|error| error.to_string())?;
    Ok(())
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
