//! Editor-independent language server configuration.
//!
//! Deserialize each input layer as [`ConfigurationSettings`] and apply layers from lowest to
//! highest priority to [`Configuration::default`]. Missing and `null` fields inherit from the
//! preceding layer. Diagnostic fields inherit individually; source discovery is replaced as a
//! whole. To restore Spago discovery over a command-based layer, supply:
//!
//! ```json
//! {"sources":{"kind":"spago"}}
//! ```
//!
//! A runtime settings snapshot replaces the previous runtime layer. Apply it to the startup
//! baseline, not to the previous effective configuration, so omitted settings stop overriding
//! that baseline. Top-level `null` represents an empty layer at the transport boundary: consumers
//! can deserialize `Option<ConfigurationSettings>` and use `unwrap_or_default()`.
//!
//! This crate owns neither configuration transport nor command execution. Enable the `schema`
//! feature to export the input contract with `schema()`. Regenerate the checked-in JSON Schema:
//!
//! ```sh
//! just configuration-schema
//! ```
//!
//! The schema defines the supported JSON representation exchanged with external producers and
//! consumers. Serializing valid configuration values produces schema-compliant JSON, and
//! deserialization accepts schema-compliant JSON produced externally. Serde may also deserialize
//! position-encoded arrays; that permissiveness is an implementation detail, not a supported
//! configuration format or a compatibility guarantee.

use serde::{Deserialize, Deserializer, Serialize, de};

/// Effective settings after resolving all configuration layers.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Configuration {
    pub sources: SourceDiscovery,
    pub diagnostics: Diagnostics,
}

/// One partial configuration layer. Missing or null fields inherit from lower-priority layers.
/// Each runtime snapshot replaces the preceding runtime layer, not the startup baseline.
/// Unknown fields are rejected, including an embedded `$schema` property; associate this schema
/// through editor settings instead.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ConfigurationSettings {
    /// Replace the complete source selection. Inherits when missing or null; defaults to Spago
    /// when no layer supplies a selection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sources: Option<SourceDiscovery>,
    /// Override diagnostic triggers individually. Missing or null leaves lower layers unchanged.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<DiagnosticSettings>,
}

/// Generate the partial input contract, including top-level null as an empty settings layer.
#[cfg(feature = "schema")]
pub fn schema() -> schemars::Schema {
    schemars::generate::SchemaSettings::draft2020_12()
        .for_deserialize()
        .into_generator()
        .into_root_schema_for::<Option<ConfigurationSettings>>()
}

impl ConfigurationSettings {
    /// Resolve this layer over a lower-priority configuration without modifying either input.
    pub fn apply_to(&self, baseline: &Configuration) -> Configuration {
        let mut configuration = baseline.clone();
        if let Some(sources) = &self.sources {
            configuration.sources = sources.clone();
        }
        if let Some(diagnostics) = &self.diagnostics {
            if let Some(on_open) = diagnostics.on_open {
                configuration.diagnostics.on_open = on_open;
            }
            if let Some(on_save) = diagnostics.on_save {
                configuration.diagnostics.on_save = on_save;
            }
            if let Some(on_change) = diagnostics.on_change {
                configuration.diagnostics.on_change = on_change;
            }
        }
        configuration
    }
}

/// How to discover project sources. Commands name an executable and arguments, not shell text.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum SourceDiscovery {
    /// Discover sources through spago.lock. Explicitly selects Spago over a lower command layer.
    Spago {},
    /// Select a source discovery command. Only supply commands from trusted configuration.
    Command {
        /// Executable name or path containing a non-whitespace character, passed unchanged without shell parsing.
        // Match Rust's Unicode White_Space set; JSON Schema's ECMAScript \s differs.
        #[cfg_attr(
            feature = "schema",
            schemars(
                length(min = 1),
                regex(
                    pattern = r"[^\u0009-\u000D\u0020\u0085\u00A0\u1680\u2000-\u200A\u2028\u2029\u202F\u205F\u3000]"
                )
            )
        )]
        program: String,
        /// Individual command arguments. Omission means no arguments, not inherited arguments.
        #[serde(default)]
        arguments: Vec<String>,
    },
}

impl Default for SourceDiscovery {
    fn default() -> SourceDiscovery {
        SourceDiscovery::Spago {}
    }
}

impl<'de> Deserialize<'de> for SourceDiscovery {
    fn deserialize<D>(deserializer: D) -> Result<SourceDiscovery, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize fields directly: Serde's internally tagged enum buffering loses the
        // input position of errors in command fields, pointing at the enclosing object instead.
        #[derive(Deserialize)]
        #[serde(variant_identifier, rename_all = "camelCase")]
        enum Kind {
            Spago,
            Command,
        }

        #[derive(Deserialize)]
        #[serde(rename = "SourceDiscovery", deny_unknown_fields)]
        struct Fields {
            kind: Kind,
            #[serde(default, deserialize_with = "deserialize_program")]
            program: Option<String>,
            #[serde(default, deserialize_with = "deserialize_arguments")]
            arguments: Option<Vec<String>>,
        }

        let fields = Fields::deserialize(deserializer)?;
        match fields.kind {
            Kind::Spago => {
                if fields.program.is_some() {
                    return Err(de::Error::unknown_field("program", &["kind"]));
                }
                if fields.arguments.is_some() {
                    return Err(de::Error::unknown_field("arguments", &["kind"]));
                }
                Ok(SourceDiscovery::Spago {})
            }
            Kind::Command => {
                let program = fields.program.ok_or_else(|| de::Error::missing_field("program"))?;
                let arguments = fields.arguments.unwrap_or_default();
                Ok(SourceDiscovery::Command { program, arguments })
            }
        }
    }
}

fn deserialize_program<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    struct ProgramVisitor;

    impl de::Visitor<'_> for ProgramVisitor {
        type Value = String;

        fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            formatter.write_str("a string")
        }

        fn visit_str<E: de::Error>(self, program: &str) -> Result<String, E> {
            self.visit_string(program.to_string())
        }

        fn visit_string<E: de::Error>(self, program: String) -> Result<String, E> {
            if program.trim().is_empty() {
                return Err(de::Error::custom(
                    "source command program must not be empty or whitespace-only",
                ));
            }
            Ok(program)
        }
    }

    // Fail inside the string visitor, before an enclosing map consumes its closing brace.
    deserializer.deserialize_string(ProgramVisitor).map(Some)
}

fn deserialize_arguments<'de, D>(deserializer: D) -> Result<Option<Vec<String>>, D::Error>
where
    D: Deserializer<'de>,
{
    Vec::<String>::deserialize(deserializer).map(Some)
}

/// Diagnostic triggers, not a global diagnostics enable/disable policy.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Diagnostics {
    pub on_open: bool,
    pub on_save: bool,
    pub on_change: bool,
}

impl Default for Diagnostics {
    fn default() -> Diagnostics {
        Diagnostics { on_open: true, on_save: true, on_change: false }
    }
}

/// Partial diagnostic overrides; `false` is an override, while missing or `null` inherits.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct DiagnosticSettings {
    /// Publish diagnostics on document open. The built-in default is true.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_open: Option<bool>,
    /// Publish diagnostics on document save. The built-in default is true.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_save: Option<bool>,
    /// Publish diagnostics on document change. The built-in default is false.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_change: Option<bool>,
}

#[cfg(test)]
mod tests;
