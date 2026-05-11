use assert_cmd::prelude::*;
use predicates::prelude::*;
use std::process::Command;
use tempfile::tempdir;

/// Returns a `Command` for `skh` with an isolated config directory so tests
/// never touch the real user config at `~/.config/skh/`.
fn skh_with_temp_home(tmp: &tempfile::TempDir) -> Command {
    let mut cmd = Command::cargo_bin("skh").unwrap();
    // `directories` on Linux respects XDG_CONFIG_HOME for the config dir.
    cmd.env("XDG_CONFIG_HOME", tmp.path().join("config"));
    // Clear any ambient API key so tests are deterministic.
    cmd.env_remove("SOLHUB_API_KEY");
    cmd.env_remove("SOLHUB_API_URL");
    cmd
}

#[test]
fn auth_status_with_empty_config() {
    let tmp = tempdir().unwrap();
    skh_with_temp_home(&tmp)
        .args(["auth", "status"])
        .assert()
        .success()
        .stdout(predicate::str::contains("API Key: <not set>"));
}

#[test]
fn config_set_and_list() {
    let tmp = tempdir().unwrap();

    skh_with_temp_home(&tmp)
        .args(["config", "set", "api_url", "http://example.com"])
        .assert()
        .success();

    skh_with_temp_home(&tmp)
        .args(["config", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("http://example.com"));
}

#[test]
fn workflow_list_unreachable_api_errors_gracefully() {
    let tmp = tempdir().unwrap();

    // Point to a definitely-unreachable port.
    skh_with_temp_home(&tmp)
        .env("SOLHUB_API_URL", "http://127.0.0.1:19999")
        .args(["workflow", "list"])
        .assert()
        .failure();
}
