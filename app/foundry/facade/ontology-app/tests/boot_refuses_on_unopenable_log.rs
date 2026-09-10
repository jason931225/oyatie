use std::path::PathBuf;

use foundry_ontology_app::{BootError, Config, compose};

fn temp_path(case: &str, slot: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "foundry-ontology-app-{case}-{slot}-{}-{}.sqlite",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock is after the epoch")
            .as_nanos()
    ))
}

fn a_path_that_cannot_be_opened(under: &std::path::Path) -> PathBuf {
    under.join("no-such-directory").join("log.sqlite")
}

struct Paths {
    action: PathBuf,
    denial: PathBuf,
}

impl Paths {
    fn new(case: &str) -> Self {
        let action = temp_path(case, "action");
        let denial = temp_path(case, "denial");
        let _ = std::fs::remove_file(&action);
        let _ = std::fs::remove_file(&denial);
        Self { action, denial }
    }

    fn config(&self) -> Config {
        Config {
            listen_addr: "127.0.0.1:0".into(),
            action_log: self.action.clone(),
            denial_log: self.denial.clone(),
            tenants: vec!["ten_test".into()],
            operators: Vec::new(),
        }
    }
}

impl Drop for Paths {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.action);
        let _ = std::fs::remove_file(&self.denial);
    }
}

#[test]
fn a_clean_configuration_boots() {
    let paths = Paths::new("clean");
    let state = compose(&paths.config()).expect("a fresh store boots");
    assert_eq!(state.tenant_count(), 1);
    assert!(
        foundry_ontology_app::observation::observe(&state).is_caught_up(),
        "an empty log has zero lag"
    );
}

#[test]
fn an_unopenable_action_log_refuses_boot() {
    let paths = Paths::new("unopenable-action");
    let mut config = paths.config();
    config.action_log = a_path_that_cannot_be_opened(&paths.action);
    let refused = compose(&config).expect_err("an unopenable action log refuses boot");
    assert!(
        matches!(refused, BootError::ActionLogUnopenable { .. }),
        "expected a typed action-log refusal, got {refused:?}"
    );
}

#[test]
fn an_unopenable_denial_log_refuses_boot() {
    let paths = Paths::new("unopenable-denial");
    let mut config = paths.config();
    config.denial_log = a_path_that_cannot_be_opened(&paths.denial);
    let refused = compose(&config).expect_err("an unopenable denial log refuses boot");
    assert!(
        matches!(refused, BootError::DenialLogUnopenable { .. }),
        "expected a typed denial-log refusal, got {refused:?}"
    );
}

#[test]
fn one_path_for_both_logs_refuses_boot() {
    let paths = Paths::new("aliased");
    let mut config = paths.config();
    config.denial_log = config.action_log.clone();
    assert_eq!(
        compose(&config).expect_err("aliased logs refuse boot"),
        BootError::LogPathsAliased
    );
}

#[test]
fn an_empty_tenant_roster_refuses_boot() {
    let paths = Paths::new("no-tenants");
    let mut config = paths.config();
    config.tenants = Vec::new();
    assert_eq!(
        compose(&config).expect_err("an empty roster refuses boot"),
        BootError::NoTenantsConfigured
    );
}

#[test]
fn an_unseedable_tenant_refuses_boot() {
    let paths = Paths::new("bad-tenant");
    let mut config = paths.config();
    config.tenants = vec!["not-a-tenant-id".into()];
    let refused = compose(&config).expect_err("an unseedable tenant refuses boot");
    assert!(
        matches!(refused, BootError::SeedRefused { .. }),
        "expected a typed seed refusal, got {refused:?}"
    );
}
