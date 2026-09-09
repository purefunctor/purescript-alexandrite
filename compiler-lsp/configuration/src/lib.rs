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
//! The schema describes named JSON objects, not Serde's positional-array struct representation.

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
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
pub enum SourceDiscovery {
    /// Discover sources through spago.lock. Explicitly selects Spago over a lower command layer.
    Spago {},
    /// Select a source discovery command. Only supply commands from trusted configuration.
    Command {
        /// Nonempty executable name or path, passed without shell parsing.
        #[serde(deserialize_with = "deserialize_program")]
        #[cfg_attr(feature = "schema", schemars(length(min = 1)))]
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

fn deserialize_program<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let program = String::deserialize(deserializer)?;
    if program.is_empty() {
        return Err(de::Error::custom("source command program must not be empty"));
    }
    Ok(program)
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
