use serde_json::{Value, json};

use super::{Configuration, ConfigurationSettings, Diagnostics, SourceDiscovery};

fn settings(value: Value) -> ConfigurationSettings {
    #[cfg(feature = "schema")]
    {
        let schema = serde_json::to_value(super::schema()).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        assert!(validator.is_valid(&value), "schema rejected {value}");
    }
    serde_json::from_value::<Option<ConfigurationSettings>>(value).unwrap().unwrap_or_default()
}

#[test]
fn defaults_match_existing_diagnostic_triggers() {
    let configuration = Configuration::default();
    assert_eq!(configuration.sources, SourceDiscovery::Spago {});
    assert_eq!(
        configuration.diagnostics,
        Diagnostics { on_open: true, on_save: true, on_change: false }
    );
    assert_eq!(
        serde_json::to_value(&configuration).unwrap(),
        json!({
            "sources": {"kind": "spago"},
            "diagnostics": {"onOpen": true, "onSave": true, "onChange": false}
        })
    );
}

#[test]
fn layers_override_only_supplied_fields() {
    let startup = settings(json!({
        "sources": {"kind": "command", "program": "custom", "arguments": ["sources"]},
        "diagnostics": {"onOpen": false, "onChange": true}
    }));
    let baseline = startup.apply_to(&Configuration::default());
    let initialization = settings(json!({"diagnostics": {"onSave": false}}));
    let baseline = initialization.apply_to(&baseline);
    let runtime = settings(json!({"diagnostics": {"onOpen": true, "onChange": false}}));
    let effective = runtime.apply_to(&baseline);

    assert_eq!(
        effective.sources,
        SourceDiscovery::Command {
            program: "custom".to_string(),
            arguments: vec!["sources".to_string()],
        }
    );
    assert_eq!(
        effective.diagnostics,
        Diagnostics { on_open: true, on_save: false, on_change: false }
    );
    assert_eq!(
        baseline.diagnostics,
        Diagnostics { on_open: false, on_save: false, on_change: true }
    );

    let replacement = settings(json!({"diagnostics": {"onSave": true}}));
    assert_eq!(
        replacement.apply_to(&baseline).diagnostics,
        Diagnostics { on_open: false, on_save: true, on_change: true }
    );
}

#[test]
fn missing_and_null_inherit_instead_of_resetting_to_defaults() {
    let baseline = Configuration {
        sources: SourceDiscovery::Command { program: "custom".to_string(), arguments: vec![] },
        diagnostics: Diagnostics { on_open: false, on_save: false, on_change: true },
    };
    for value in [
        json!(null),
        json!({}),
        json!({"sources": null, "diagnostics": null}),
        json!({"diagnostics": {}}),
        json!({"diagnostics": {"onOpen": null, "onSave": null, "onChange": null}}),
    ] {
        assert_eq!(settings(value.clone()).apply_to(&baseline), baseline, "{value}");
    }
}

#[test]
fn source_selection_is_replaced_as_a_whole() {
    let baseline = Configuration {
        sources: SourceDiscovery::Command {
            program: "old".to_string(),
            arguments: vec!["old-argument".to_string()],
        },
        ..Configuration::default()
    };
    let command = settings(json!({"sources": {"kind": "command", "program": "new"}}));
    assert_eq!(
        command.apply_to(&baseline).sources,
        SourceDiscovery::Command { program: "new".to_string(), arguments: vec![] }
    );
    let spago = settings(json!({"sources": {"kind": "spago"}}));
    assert_eq!(spago.apply_to(&baseline).sources, SourceDiscovery::Spago {});
}

#[test]
fn malformed_settings_are_rejected() {
    #[cfg(feature = "schema")]
    let validator =
        jsonschema::validator_for(&serde_json::to_value(super::schema()).unwrap()).unwrap();

    for value in [
        json!(false),
        json!("settings"),
        json!({"$schema": "https://example.com/schema.json"}),
        json!({"unknown": true}),
        json!({"diagnostics": {"onOpened": true}}),
        json!({"diagnostics": {"onOpen": "false"}}),
        json!({"sources": {}}),
        json!({"sources": {"kind": "unknown"}}),
        json!({"sources": {"kind": "spago", "program": "unexpected"}}),
        json!({"sources": {"kind": "command"}}),
        json!({"sources": {"kind": "command", "program": null}}),
        json!({"sources": {"kind": "command", "program": "custom", "arguments": null}}),
        json!({"sources": {"kind": "command", "program": "custom", "arguments": [1]}}),
        json!({"sources": {"kind": "command", "program": "custom", "shell": true}}),
    ] {
        assert!(serde_json::from_value::<ConfigurationSettings>(value.clone()).is_err(), "{value}");
        #[cfg(feature = "schema")]
        assert!(!validator.is_valid(&value), "schema accepted {value}");
    }
}

#[test]
fn source_programs_require_non_whitespace_without_trimming() {
    #[cfg(feature = "schema")]
    let validator =
        jsonschema::validator_for(&serde_json::to_value(super::schema()).unwrap()).unwrap();

    for program in ["   ", "", "\t\n\u{000b}\u{000c}\r", "\u{0085}", "\u{00a0}", "\u{3000}"] {
        let value = json!({"sources": {"kind": "command", "program": program}});
        let error = serde_json::from_value::<ConfigurationSettings>(value.clone()).unwrap_err();
        assert_eq!(
            error.to_string(),
            "source command program must not be empty or whitespace-only"
        );
        #[cfg(feature = "schema")]
        assert!(!validator.is_valid(&value), "schema accepted {value}");
    }

    for program in ["spago", " \t/path with spaces/executable\u{3000}", "\u{feff}"] {
        let value = json!({"sources": {"kind": "command", "program": program}});
        assert_eq!(
            settings(value).sources,
            Some(SourceDiscovery::Command { program: program.to_string(), arguments: vec![] })
        );
    }
}

#[test]
fn serialization_preserves_overrides_and_command_arguments() {
    let value = json!({
        "sources": {
            "kind": "command",
            "program": "/path with spaces/executable",
            "arguments": ["", "path with spaces", "--flag", "λ"]
        },
        "diagnostics": {"onOpen": false, "onChange": true}
    });
    let configuration = settings(value.clone());
    assert_eq!(serde_json::to_value(&configuration).unwrap(), value);
    assert_eq!(serde_json::to_value(ConfigurationSettings::default()).unwrap(), json!({}));

    let effective = configuration.apply_to(&Configuration::default());
    let exported = serde_json::to_value(&effective).unwrap();
    assert_eq!(settings(exported).apply_to(&Configuration::default()), effective);
}

#[cfg(feature = "schema")]
#[test]
fn schema_excludes_serdes_positional_struct_representation() {
    let validator =
        jsonschema::validator_for(&serde_json::to_value(super::schema()).unwrap()).unwrap();
    for value in [json!([]), json!({"diagnostics": []})] {
        assert!(serde_json::from_value::<ConfigurationSettings>(value.clone()).is_ok());
        assert!(!validator.is_valid(&value), "{value}");
    }
}

#[cfg(feature = "schema")]
#[test]
fn checked_in_schema_matches_generated_schema() {
    let generated = serde_json::to_string_pretty(&super::schema()).unwrap();
    assert_eq!(
        include_str!("../configuration.schema.json"),
        format!("{generated}\n"),
        "run just configuration-schema"
    );
}
