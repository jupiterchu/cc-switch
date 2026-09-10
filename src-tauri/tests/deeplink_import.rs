use std::sync::Arc;

use base64::prelude::*;
use serde_json::json;

use cc_switch_lib::{
    import_provider_from_deeplink, parse_deeplink_url, AppState, AppType, Database, ProviderService,
};

#[path = "support.rs"]
mod support;
use support::{ensure_test_home, reset_test_fs, test_mutex};

#[test]
fn deeplink_import_claude_provider_persists_to_db() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();

    let url = "ccswitch://v1/import?resource=provider&app=claude&name=DeepLink%20Claude&homepage=https%3A%2F%2Fexample.com&endpoint=https%3A%2F%2Fapi.example.com%2Fv1&apiKey=sk-test-claude-key&model=claude-sonnet-4&icon=claude";
    let request = parse_deeplink_url(url).expect("parse deeplink url");

    let db = Arc::new(Database::memory().expect("create memory db"));
    let state = AppState::new(db.clone());

    let provider_id = import_provider_from_deeplink(&state, request.clone())
        .expect("import provider from deeplink");

    // Verify DB state
    let providers = db.get_all_providers("claude").expect("get providers");
    let provider = providers
        .get(&provider_id)
        .expect("provider created via deeplink");

    assert_eq!(provider.name, request.name.clone().unwrap());
    assert_eq!(provider.website_url.as_deref(), request.homepage.as_deref());
    assert_eq!(provider.icon.as_deref(), Some("claude"));
    let auth_token = provider
        .settings_config
        .pointer("/env/ANTHROPIC_AUTH_TOKEN")
        .and_then(|v| v.as_str());
    let base_url = provider
        .settings_config
        .pointer("/env/ANTHROPIC_BASE_URL")
        .and_then(|v| v.as_str());
    assert_eq!(auth_token, request.api_key.as_deref());
    assert_eq!(base_url, request.endpoint.as_deref());
}

#[test]
fn deeplink_import_codex_provider_builds_auth_and_config() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();

    let url = "ccswitch://v1/import?resource=provider&app=codex&name=DeepLink%20Codex&homepage=https%3A%2F%2Fopenai.example&endpoint=https%3A%2F%2Fapi.openai.example%2Fv1&apiKey=sk-test-codex-key&model=gpt-4o&icon=openai";
    let request = parse_deeplink_url(url).expect("parse deeplink url");

    let db = Arc::new(Database::memory().expect("create memory db"));
    let state = AppState::new(db.clone());

    let provider_id = import_provider_from_deeplink(&state, request.clone())
        .expect("import provider from deeplink");

    let providers = db.get_all_providers("codex").expect("get providers");
    let provider = providers
        .get(&provider_id)
        .expect("provider created via deeplink");

    assert_eq!(provider.name, request.name.clone().unwrap());
    assert_eq!(provider.website_url.as_deref(), request.homepage.as_deref());
    assert_eq!(provider.icon.as_deref(), Some("openai"));
    let auth_value = provider
        .settings_config
        .pointer("/auth/OPENAI_API_KEY")
        .and_then(|v| v.as_str());
    let config_text = provider
        .settings_config
        .get("config")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    assert_eq!(auth_value, request.api_key.as_deref());
    let config: toml::Value = toml::from_str(config_text).expect("valid legacy config");
    let custom = &config["model_providers"]["custom"];
    assert_eq!(custom["requires_openai_auth"].as_bool(), Some(true));
    assert!(custom.get("http_headers").is_none());
    assert!(custom.get("experimental_bearer_token").is_none());
    assert!(
        config_text.contains(request.endpoint.as_deref().unwrap()),
        "config.toml content should contain endpoint"
    );
    assert!(
        config_text.contains("model = \"gpt-4o\""),
        "config.toml content should contain model setting"
    );
}

fn codex_config_deeplink(config: serde_json::Value, overrides: &[(&str, &str)]) -> String {
    let encoded = BASE64_STANDARD.encode(serde_json::to_vec(&config).unwrap());
    let mut url = url::Url::parse("ccswitch://v1/import").unwrap();
    url.query_pairs_mut()
        .extend_pairs([
            ("resource", "provider"),
            ("app", "codex"),
            ("name", "AIGoCode 图片 \"Relay\""),
            ("configFormat", "json"),
            ("config", encoded.as_str()),
        ])
        .extend_pairs(overrides.iter().copied());
    url.to_string()
}

#[test]
fn deeplink_import_codex_preserves_image_options_in_live_config() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let home = ensure_test_home();
    // Website payloads can omit auth/base URL/model and keep existing URL fields.
    let url = codex_config_deeplink(
        json!({ "config": r#"
model_provider = "aigocode"
[model_providers.aigocode]
requires_openai_auth = false
http_headers = { "x-openai-actor-authorization" = "local-relay" }
"# }),
        &[
            ("apiKey", "sk-url-test"),
            ("endpoint", "https://api.example.com/v1/"),
            ("model", "gpt-6-astra"),
            ("enabled", "true"),
        ],
    );
    let request = parse_deeplink_url(&url).expect("parse minimal Codex payload");
    let db = Arc::new(Database::memory().expect("create memory db"));
    let state = AppState::new(db.clone());
    let id =
        import_provider_from_deeplink(&state, request).expect("import and enable Codex provider");
    let providers = db.get_all_providers("codex").unwrap();
    let settings = &providers[&id].settings_config;
    let stored: toml::Value = toml::from_str(settings["config"].as_str().unwrap()).unwrap();
    let live: toml::Value = toml::from_str(
        &std::fs::read_to_string(home.join(".codex/config.toml")).expect("read live Codex config"),
    )
    .unwrap();
    for document in [&stored, &live] {
        assert_eq!(document["model_provider"].as_str(), Some("custom"));
        assert_eq!(document["model"].as_str(), Some("gpt-6-astra"));
        let provider = &document["model_providers"]["custom"];
        assert_eq!(
            provider["base_url"].as_str(),
            Some("https://api.example.com/v1")
        );
        assert_eq!(provider["requires_openai_auth"].as_bool(), Some(false));
        assert_eq!(
            provider["experimental_bearer_token"].as_str(),
            Some("sk-url-test")
        );
        assert_eq!(
            provider["http_headers"]["x-openai-actor-authorization"].as_str(),
            Some("local-relay")
        );
    }
    assert_eq!(settings["auth"]["OPENAI_API_KEY"], "sk-url-test");

    // Switching away backfills the live token into auth.json-shaped storage.
    // Switching back must project it again when OpenAI auth is disabled.
    let other_url = "ccswitch://v1/import?resource=provider&app=codex&name=Other&endpoint=https%3A%2F%2Fother.example%2Fv1&apiKey=sk-other&enabled=true";
    import_provider_from_deeplink(&state, parse_deeplink_url(other_url).unwrap()).unwrap();
    ProviderService::switch(&state, AppType::Codex, &id).expect("switch back to image provider");
    let restored: toml::Value =
        toml::from_str(&std::fs::read_to_string(home.join(".codex/config.toml")).unwrap()).unwrap();
    let provider = &restored["model_providers"]["custom"];
    assert_eq!(provider["requires_openai_auth"].as_bool(), Some(false));
    assert_eq!(
        provider["experimental_bearer_token"].as_str(),
        Some("sk-url-test")
    );
    assert_eq!(
        provider["http_headers"]["x-openai-actor-authorization"].as_str(),
        Some("local-relay")
    );
}

#[test]
fn deeplink_import_codex_url_overrides_preserve_scalars_and_exclude_untrusted_config() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();
    let config = r#"
model_provider = 'relay "图片"'
model = "payload-model"
model_reasoning_effort = "low"
sandbox_mode = "danger-full-access"
[features]
image_generation = true
[hooks]
command = "do-not-import"
[mcp_servers.do_not_import]
command = "do-not-run"
[model_providers.a_first]
base_url = "https://wrong.example/v1"
requires_openai_auth = true
http_headers = { "x-wrong-provider" = "wrong" }
[model_providers.'relay "图片"']
base_url = "https://payload.example/v1"
requires_openai_auth = false
experimental_bearer_token = "sk-payload-token"
http_headers = { "x-openai-actor-authorization" = 'relay "quoted"', "x-extra" = "kept" }
env_http_headers = { "x-secret" = "LOCAL_SECRET" }
"#;
    let model = "gpt-\"quoted\"\n[mcp_servers.injected]\ncommand=\"do-not-run\"";
    let key = "sk-url-\"quoted\"\\key";
    let url = codex_config_deeplink(
        json!({
            "auth": { "OPENAI_API_KEY": "sk-payload-auth" },
            "config": config,
        }),
        &[
            ("apiKey", key),
            ("endpoint", "https://url.example/v1/"),
            ("model", model),
        ],
    );
    let db = Arc::new(Database::memory().unwrap());
    let state = AppState::new(db.clone());
    let id = import_provider_from_deeplink(&state, parse_deeplink_url(&url).unwrap()).unwrap();
    let providers = db.get_all_providers("codex").unwrap();
    let settings = &providers[&id].settings_config;
    let document: toml::Value = toml::from_str(settings["config"].as_str().unwrap()).unwrap();
    let provider = &document["model_providers"]["custom"];
    assert_eq!(settings["auth"]["OPENAI_API_KEY"], key);
    assert_eq!(provider["experimental_bearer_token"].as_str(), Some(key));
    assert_eq!(document["model"].as_str(), Some(model));
    assert_eq!(
        provider["base_url"].as_str(),
        Some("https://url.example/v1")
    );
    assert_eq!(provider["name"].as_str(), Some("AIGoCode 图片 \"Relay\""));
    assert_eq!(
        provider["http_headers"]["x-openai-actor-authorization"].as_str(),
        Some("relay \"quoted\"")
    );
    assert_eq!(provider["http_headers"]["x-extra"].as_str(), Some("kept"));
    assert!(provider.get("env_http_headers").is_none());
    assert_eq!(document["model_providers"].as_table().unwrap().len(), 1);
    assert_eq!(document["model_reasoning_effort"].as_str(), Some("high"));
    for forbidden in ["sandbox_mode", "features", "hooks", "mcp_servers"] {
        assert!(
            document.get(forbidden).is_none(),
            "unexpected imported {forbidden}"
        );
    }
}

#[test]
fn deeplink_import_codex_fills_fields_from_the_selected_provider() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();
    let url = codex_config_deeplink(
        json!({ "config": r#"
model_provider = "z_selected"
model = "payload-model"
[model_providers.a_wrong]
base_url = "https://wrong.example/v1"
experimental_bearer_token = "sk-wrong-provider"
requires_openai_auth = false
http_headers = { "x-wrong" = "wrong" }
[model_providers.z_selected]
base_url = "https://selected.example/v1"
experimental_bearer_token = "sk-selected"
http_headers = { "x-selected" = "selected" }
"# }),
        &[],
    );
    let db = Arc::new(Database::memory().unwrap());
    let state = AppState::new(db.clone());
    let id = import_provider_from_deeplink(&state, parse_deeplink_url(&url).unwrap()).unwrap();
    let providers = db.get_all_providers("codex").unwrap();
    let settings = &providers[&id].settings_config;
    let document: toml::Value = toml::from_str(settings["config"].as_str().unwrap()).unwrap();
    let provider = &document["model_providers"]["custom"];
    assert_eq!(document["model"].as_str(), Some("payload-model"));
    assert_eq!(
        provider["base_url"].as_str(),
        Some("https://selected.example/v1")
    );
    assert_eq!(
        provider["http_headers"]["x-selected"].as_str(),
        Some("selected")
    );
    assert_eq!(provider["requires_openai_auth"].as_bool(), Some(true));
    assert!(provider.get("experimental_bearer_token").is_none());
    assert_eq!(settings["auth"]["OPENAI_API_KEY"], "sk-selected");
}

#[test]
fn deeplink_import_codex_rejects_invalid_options_without_persisting_or_echoing_keys() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();
    let db = Arc::new(Database::memory().unwrap());
    let state = AppState::new(db.clone());
    let prefix = "model_provider = 'custom'\n[model_providers.custom]\n";
    let cases = [
        json!({ "config": format!("{prefix}requires_openai_auth = 'false'") }),
        json!({ "config": format!("{prefix}http_headers = ['invalid']") }),
        json!({ "config": format!("{prefix}http_headers = {{ 'x-relay' = 1 }}") }),
        json!({ "config": format!("{prefix}http_headers = {{ 'invalid name' = 'value' }}") }),
        json!({ "config": format!("{prefix}http_headers = {{ 'x-relay' = \"bad\\r\\nheader\" }}") }),
        json!({ "config": "model_provider = 'missing'\n[model_providers.custom]" }),
        json!({ "config": "model_provider = 'oss'" }),
        json!({ "config": "model_provider = 'ollama-chat'" }),
        json!({ "config": "model_provider = 'OpenAI'" }),
        json!({ "config": "experimental_bearer_token = 'sk-never-echo'\ninvalid = [" }),
        json!({ "config": { "requires_openai_auth": false } }),
        json!(null),
        json!([]),
        json!(true),
    ];
    for config in cases {
        let url = codex_config_deeplink(
            config,
            &[
                ("apiKey", "sk-never-echo"),
                ("endpoint", "https://api.example.com/v1"),
            ],
        );
        let request = parse_deeplink_url(&url).unwrap();
        let preview_error = cc_switch_lib::merge_deeplink_config(request.clone()).unwrap_err();
        assert!(!preview_error.to_string().contains("sk-never-echo"));
        let error = import_provider_from_deeplink(&state, request).unwrap_err();
        assert!(!error.to_string().contains("sk-never-echo"));
        assert!(db.get_all_providers("codex").unwrap().is_empty());
    }
}

#[test]
fn deeplink_import_codex_builtin_providers_do_not_require_explicit_tables() {
    let _guard = test_mutex().lock().expect("acquire test mutex");
    reset_test_fs();
    let _home = ensure_test_home();
    let db = Arc::new(Database::memory().unwrap());
    let state = AppState::new(db.clone());
    for builtin in [
        "openai",
        "amazon-bedrock",
        "amazon-bedrock-runtime",
        "ollama",
        "lmstudio",
    ] {
        let url = codex_config_deeplink(
            json!({
                "config": format!("model_provider = '{builtin}'\nmodel = 'payload-model'\n"),
            }),
            &[
                ("name", builtin),
                ("apiKey", "sk-builtin-import"),
                ("endpoint", "https://api.example.com/v1"),
            ],
        );
        let id = import_provider_from_deeplink(&state, parse_deeplink_url(&url).unwrap())
            .expect("import basic configuration for a built-in provider");
        let providers = db.get_all_providers("codex").unwrap();
        let document: toml::Value =
            toml::from_str(providers[&id].settings_config["config"].as_str().unwrap()).unwrap();
        assert_eq!(document["model_provider"].as_str(), Some("custom"));
        assert_eq!(document["model"].as_str(), Some("payload-model"));
        let custom = &document["model_providers"]["custom"];
        assert_eq!(custom["requires_openai_auth"].as_bool(), Some(true));
        assert!(custom.get("http_headers").is_none());
        assert!(custom.get("experimental_bearer_token").is_none());
    }
}
