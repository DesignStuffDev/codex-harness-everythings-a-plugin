use std::fs;
use std::path::Path;
use std::process::Command;

use base64::Engine;
use codex_config::ManagedAuthPolicy;
use codex_protocol::config_types::ForcedLoginMethod;
use codex_protocol::shell_environment::OPENAI_FEDERATION_RULE_ID_ENV_VAR;
use pretty_assertions::assert_eq;
use serde_json::Value;
use serde_json::json;
use tempfile::TempDir;

use super::*;
use crate::AuthConfig;
use crate::AuthCredentialsStoreMode;
use crate::AuthKeyringBackendKind;
use crate::AuthManager;

// Cargo builds this outside the harness workspace; Bazel supplies the same std-only fixture.
// Both install an independent executable and remove the temporary source/package directory.
// Responses contain only dummy credentials. The request log never records response tokens.
const PLUGIN: &str = include_str!("../tests/fixtures/component_auth_plugin.rs");

struct Fixture {
    root: TempDir,
}

impl Fixture {
    fn new(resolve: Value, refresh: Value) -> Self {
        let root = tempfile::tempdir().expect("temporary independent auth package");
        let package = root.path().join("package");
        fs::create_dir(&package).expect("create package directory");
        let source = root.path().join("plugin.rs");
        fs::write(&source, PLUGIN).expect("write standalone plugin");
        let executable = format!("plugin{}", std::env::consts::EXE_SUFFIX);
        if option_env!("BAZEL_PACKAGE").is_some() {
            fs::copy(
                codex_utils_cargo_bin::cargo_bin("component-auth-test-plugin")
                    .expect("resolve Bazel auth fixture"),
                package.join(&executable),
            )
            .expect("copy standalone auth fixture");
        } else {
            let output = Command::new("rustc")
                .args(["--edition=2024"])
                .arg(&source)
                .arg("-o")
                .arg(package.join(&executable))
                .output()
                .expect("compile independent std-only auth provider");
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        fs::write(package.join("codex-component.json"), serde_json::to_vec(&json!({
            "api_version":1, "id":"fixture-auth", "version":"1.0.0",
            "entrypoint":executable,
            "args":[root.path().join("requests.jsonl"), root.path().join("resolve.json"), root.path().join("refresh.json")],
            "components":[{"kind":"auth","name":"default","contract_version":1}]
        })).expect("manifest JSON")).expect("write manifest");
        let fixture = Self { root };
        fixture.respond("resolve", resolve);
        fixture.respond("refresh", refresh);
        codex_component_host::install(fixture.root.path(), &package).expect("install package");
        codex_component_host::select(fixture.root.path(), "auth", "default", Some("fixture-auth"))
            .expect("select credential provider");
        fs::remove_dir_all(package).expect("remove build package");
        fs::remove_file(source).expect("remove build source");
        fixture
    }

    fn respond(&self, method: &str, result: Value) {
        fs::write(
            self.root.path().join(format!("{method}.json")),
            serde_json::to_vec(&json!({"type":"result","id":1,"result":result}))
                .expect("response JSON"),
        )
        .expect("write response fixture");
    }

    fn config(&self) -> AuthConfig {
        auth_config(self.root.path())
    }
}

fn auth_config(path: &Path) -> AuthConfig {
    AuthConfig {
        codex_home: path.to_path_buf(),
        auth_credentials_store_mode: AuthCredentialsStoreMode::File,
        keyring_backend_kind: AuthKeyringBackendKind::Direct,
        forced_login_method: None,
        chatgpt_base_url: None,
        forced_chatgpt_workspace_id: None,
        managed_auth_policy: ManagedAuthPolicy::default(),
        auth_route_config: crate::test_support::transport_default_auth_route_config(),
    }
}

fn chatgpt(account: &str, marker: &str) -> Value {
    let payload = json!({"exp":4102444800i64,"fixture":marker,
        "https://api.openai.com/auth":{"chatgpt_account_id":account,"chatgpt_user_id":"fixture-user"}});
    let token = format!(
        "e30.{}.c2ln",
        base64::engine::general_purpose::URL_SAFE_NO_PAD
            .encode(serde_json::to_vec(&payload).expect("dummy JWT payload"))
    );
    json!({"type":"chatgpt","access_token":token,"chatgpt_account_id":account,"chatgpt_plan_type":"enterprise"})
}

#[tokio::test]
async fn installed_component_resolves_and_refreshes_native_api_auth() {
    let fixture = Fixture::new(
        json!({"type":"api_key","api_key":"dummy-first"}),
        json!({"type":"api_key","api_key":"dummy-refreshed"}),
    );
    let manager = AuthManager::shared_from_auth_config(
        fixture.config(),
        /*enable_codex_api_key_env*/ false,
    )
    .await
    .expect("initialize selected provider");
    assert_eq!(
        manager.auth_cached().expect("cached auth").api_key(),
        Some("dummy-first")
    );
    assert!(manager.has_external_auth());
    manager
        .refresh_token_from_authority()
        .await
        .expect("refresh native manager");
    let auth = manager.auth_cached().expect("refreshed auth");
    assert_eq!(auth.api_key(), Some("dummy-refreshed"));
    assert!(!format!("{auth:?} {manager:?}").contains("dummy-"));
    assert!(!fixture.root.path().join("auth.json").exists());
    let requests: Vec<Value> = fs::read_to_string(fixture.root.path().join("requests.jsonl"))
        .expect("requests")
        .lines()
        .map(|line| serde_json::from_str(line).expect("request JSON"))
        .collect();
    assert_eq!(
        requests,
        vec![
            json!({"type":"request","id":1,"component":{"kind":"auth","name":"default"},"method":"auth.resolve","params":{}}),
            json!({"type":"request","id":1,"component":{"kind":"auth","name":"default"},"method":"auth.refresh","params":{"reason":"unauthorized","previous_account_id":null}}),
        ]
    );
}

#[tokio::test]
async fn chatgpt_refresh_preserves_account_and_rejects_owner_change() {
    let first = chatgpt("workspace-one", "first");
    let refreshed = chatgpt("workspace-one", "refreshed");
    let fixture = Fixture::new(first.clone(), refreshed.clone());
    let manager = AuthManager::shared_from_auth_config(
        fixture.config(),
        /*enable_codex_api_key_env*/ false,
    )
    .await
    .expect("initialize ChatGPT provider");
    manager
        .refresh_token_from_authority()
        .await
        .expect("refresh same account");
    let auth = manager.auth_cached().expect("cached auth");
    assert_eq!(
        (auth.get_token().expect("token"), auth.get_account_id()),
        (
            refreshed["access_token"]
                .as_str()
                .expect("fixture token")
                .to_owned(),
            Some("workspace-one".to_owned())
        )
    );
    for secret in [
        first["access_token"].as_str().unwrap(),
        refreshed["access_token"].as_str().unwrap(),
    ] {
        assert!(!format!("{auth:?} {manager:?}").contains(secret));
    }
    fixture.respond("refresh", chatgpt("workspace-two", "different-owner"));
    let error = manager
        .refresh_token_from_authority()
        .await
        .expect_err("account change rejected");
    assert!(matches!(error, RefreshTokenError::Permanent(_)));
    assert_eq!(
        manager
            .auth_cached()
            .expect("original cached auth")
            .get_token()
            .unwrap(),
        auth.get_token().unwrap()
    );
    assert!(!fixture.root.path().join("auth.json").exists());
    let requests =
        fs::read_to_string(fixture.root.path().join("requests.jsonl")).expect("requests");
    let refresh: Value =
        serde_json::from_str(requests.lines().nth(1).expect("refresh request")).unwrap();
    assert_eq!(
        refresh["params"],
        json!({"reason":"unauthorized","previous_account_id":"workspace-one"})
    );
}

#[tokio::test]
async fn selected_credentials_cannot_bypass_native_login_or_workspace_policy() {
    for (resolve, methods, workspaces) in [
        (
            json!({"type":"api_key","api_key":"dummy-policy-secret"}),
            Some(vec![ForcedLoginMethod::Chatgpt]),
            None,
        ),
        (
            chatgpt("wrong-workspace", "policy"),
            None,
            Some(vec!["required-workspace".to_owned()]),
        ),
    ] {
        let fixture = Fixture::new(resolve, json!({}));
        let mut config = fixture.config();
        config.managed_auth_policy.allowed_login_methods = methods;
        config.forced_chatgpt_workspace_id = workspaces;
        let error =
            AuthManager::shared_from_auth_config(config, /*enable_codex_api_key_env*/ false)
                .await
                .expect_err("native authentication restrictions must hold");
        assert_eq!(
            error.to_string(),
            "component credentials were rejected by native authentication policy"
        );
        assert!(!format!("{error:?}").contains("dummy-policy-secret"));
    }
}

#[tokio::test]
async fn provider_failures_are_classified_without_credential_diagnostics() {
    let fixture = Fixture::new(json!({"type":"api_key","api_key":"dummy-valid"}), json!({}));
    let provider = ComponentAuth::selected(fixture.root.path())
        .expect("catalog")
        .expect("selected");
    for (response, transient) in [
        (json!({"type":"error","kind":"transient"}), true),
        (json!({"type":"error","kind":"permanent"}), false),
        (
            json!({"type":"dummy-response-secret","api_key":"dummy-response-secret"}),
            false,
        ),
        (
            json!({"type":"chatgpt","access_token":"dummy-invalid-token","chatgpt_account_id":"fixture","chatgpt_plan_type":null}),
            false,
        ),
    ] {
        fixture.respond("resolve", response);
        let error =
            provider.classify_error(provider.resolve().await.expect_err("provider failure"));
        assert_eq!(matches!(error, RefreshTokenError::Transient(_)), transient);
        assert!(!format!("{error:?} {error}").contains("dummy-"));
    }
    fs::write(
        fixture.root.path().join("resolve.json"),
        r#"{"type":"error","id":1,"message":"dummy-transport-secret"}"#,
    )
    .unwrap();
    let error = provider.classify_error(provider.resolve().await.expect_err("transport error"));
    assert!(!format!("{error:?} {error} {provider:?}").contains("dummy-transport-secret"));
}

#[tokio::test]
async fn deselection_keeps_native_authentication_and_does_not_start_provider() {
    let fixture = Fixture::new(
        json!({"type":"api_key","api_key":"dummy-plugin"}),
        json!({}),
    );
    codex_component_host::select(
        fixture.root.path(),
        "auth",
        "default",
        /*plugin_id*/ None,
    )
    .unwrap();
    crate::login_with_api_key(
        fixture.root.path(),
        "dummy-native",
        AuthCredentialsStoreMode::File,
        AuthKeyringBackendKind::Direct,
    )
    .expect("seed native credential store");
    let manager = AuthManager::shared_from_auth_config(
        fixture.config(),
        /*enable_codex_api_key_env*/ false,
    )
    .await
    .expect("native default");
    assert!(!manager.has_external_auth());
    assert_eq!(
        manager.auth_cached().expect("native auth").api_key(),
        Some("dummy-native")
    );
    assert!(!fixture.root.path().join("requests.jsonl").exists());
}

#[tokio::test]
async fn selected_component_cannot_replace_workload_identity() {
    const CHILD_HOME: &str = "CODEX_COMPONENT_AUTH_TEST_CHILD_HOME";
    if let Some(path) = std::env::var_os(CHILD_HOME) {
        let path = std::path::PathBuf::from(path);
        let error = AuthManager::shared_from_auth_config(
            auth_config(&path),
            /*enable_codex_api_key_env*/ false,
        )
        .await
        .expect_err("workload identity retains ownership");
        assert_eq!(
            error.to_string(),
            "workload identity auth cannot be replaced by a component"
        );
        assert!(!path.join("requests.jsonl").exists());
        return;
    }
    let fixture = Fixture::new(
        json!({"type":"api_key","api_key":"dummy-unused"}),
        json!({}),
    );
    let output = Command::new(std::env::current_exe().expect("test binary"))
        .args([
            "--exact",
            "component_auth::tests::selected_component_cannot_replace_workload_identity",
            "--nocapture",
        ])
        .env(CHILD_HOME, fixture.root.path())
        .env(OPENAI_FEDERATION_RULE_ID_ENV_VAR, "fixture-rule")
        .output()
        .expect("run isolated workload identity check");
    assert!(
        output.status.success(),
        "{} {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
