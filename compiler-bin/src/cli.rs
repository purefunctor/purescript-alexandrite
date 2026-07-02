use std::path::PathBuf;

use clap::{ArgAction, Args, Parser, Subcommand};
use tracing::level_filters::LevelFilter;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Debug, Parser)]
#[command(about, version(VERSION))]
pub struct Cli {
    /// Print log path.
    #[arg(long)]
    pub log_file: bool,
    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run the language server.
    Lsp(LspOptions),
    /// Documentation utilities.
    Docs(DocsOptions),
}

#[derive(Debug, Args)]
pub struct LoggingOptions {
    /// Log level for the query engine.
    #[arg(long, value_name("LevelFilter"), default_value("off"))]
    pub query_log: LevelFilter,

    /// Log level for the type checker.
    #[arg(long, value_name("LevelFilter"), default_value("off"))]
    pub checking_log: LevelFilter,
}

#[derive(Debug, Args)]
pub struct LspOptions {
    #[command(flatten)]
    pub logging: LoggingOptions,

    #[arg(long)]
    pub stdio: bool,

    /// Log level for the language server.
    #[arg(long, value_name("LevelFilter"), default_value("info"))]
    pub lsp_log: LevelFilter,

    /// Command to use to get source files.
    ///
    /// This argument also disables the spago.lock integration.
    #[arg(long)]
    pub source_command: Option<String>,

    /// Publish diagnostics on textDocument/didOpen.
    #[arg(long, value_name("bool"), action = ArgAction::Set, default_value_t = true)]
    pub diagnostics_on_open: bool,

    /// Publish diagnostics on textDocument/didSave.
    #[arg(long, value_name("bool"), action = ArgAction::Set, default_value_t = true)]
    pub diagnostics_on_save: bool,

    /// Publish diagnostics on textDocument/didChange.
    #[arg(long, default_value_t = false)]
    pub diagnostics_on_change: bool,
}

#[derive(Debug, Args)]
#[command(subcommand_negates_reqs = true)]
pub struct DocsOptions {
    #[command(flatten)]
    pub logging: LoggingOptions,
    /// Log level for the documentation tool.
    #[arg(long, value_name("LEVEL"), default_value("info"))]
    pub docs_log: LevelFilter,
    #[command(subcommand)]
    pub command: Option<DocsCommand>,
    /// Output directory for the generated documentation.
    #[arg(long, value_name("DIR"), default_value("docs"))]
    pub output: PathBuf,
    /// Spago project directory containing spago.lock.
    #[arg(long, value_name("DIR"), conflicts_with("package"))]
    pub spago_project: Option<PathBuf>,
    /// Package folder to document.
    #[arg(
        id = "package",
        long = "package",
        value_name("DIR"),
        required_unless_present("spago_project")
    )]
    pub packages: Vec<PathBuf>,
}

#[derive(Debug, Subcommand)]
pub enum DocsCommand {
    /// Generate TypeScript declarations for the documentation JSON schema.
    #[command(name = "typescript")]
    TypeScript(DocsTypeScriptOptions),
}

#[derive(Debug, Args)]
pub struct DocsTypeScriptOptions {
    /// Output directory for the generated TypeScript schema.
    #[arg(long, value_name("DIR"), default_value("src-generated"))]
    pub output: PathBuf,
}

impl Default for LoggingOptions {
    fn default() -> LoggingOptions {
        LoggingOptions { query_log: LevelFilter::OFF, checking_log: LevelFilter::OFF }
    }
}

impl Default for LspOptions {
    fn default() -> LspOptions {
        LspOptions {
            logging: LoggingOptions::default(),
            stdio: false,
            lsp_log: LevelFilter::INFO,
            source_command: None,
            diagnostics_on_open: true,
            diagnostics_on_save: true,
            diagnostics_on_change: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::error::ErrorKind;

    fn docs(args: &[&str]) -> DocsOptions {
        let mut argv = vec!["alexandrite", "docs"];
        argv.extend(args);
        let cli = Cli::parse_from(argv);
        match cli.command {
            Some(Command::Docs(options)) => options,
            _ => unreachable!("parsed command was not `docs`"),
        }
    }

    fn docs_error_kind(args: &[&str]) -> ErrorKind {
        let mut argv = vec!["alexandrite", "docs"];
        argv.extend(args);
        Cli::try_parse_from(argv).unwrap_err().kind()
    }

    fn typescript(args: &[&str]) -> DocsTypeScriptOptions {
        match docs(args).command {
            Some(DocsCommand::TypeScript(options)) => options,
            _ => unreachable!("parsed command was not `typescript`"),
        }
    }

    #[test]
    fn single_package_folder() {
        let options = docs(&["--package", "packages/effect"]);
        insta::assert_debug_snapshot!(options.packages, @r#"
        [
            "packages/effect",
        ]
        "#);
    }

    #[test]
    fn repeated_packages_keep_order() {
        let options = docs(&["--package", "packages/effect", "--package", "packages/prelude"]);
        insta::assert_debug_snapshot!(options.packages, @r#"
        [
            "packages/effect",
            "packages/prelude",
        ]
        "#);
    }

    #[test]
    fn spago_project_replaces_package_specs() {
        let options = docs(&["--spago-project", "."]);
        insta::assert_debug_snapshot!((&options.spago_project, &options.packages), @r#"
        (
            Some(
                ".",
            ),
            [],
        )
        "#);
    }

    #[test]
    fn spago_project_conflicts_with_package_specs() {
        insta::assert_debug_snapshot!(
            docs_error_kind(&["--spago-project", ".", "--package", "packages/effect"]),
            @"ArgumentConflict"
        );
    }

    #[test]
    fn later_flags_are_not_consumed_as_package_paths() {
        let options = docs(&["--package", "packages/effect", "--output", "out"]);
        insta::assert_debug_snapshot!((&options.output, &options.packages), @r#"
        (
            "out",
            [
                "packages/effect",
            ],
        )
        "#);
    }

    #[test]
    fn missing_package_path_is_rejected() {
        insta::assert_debug_snapshot!(docs_error_kind(&["--package"]), @"InvalidValue");
    }

    #[test]
    fn package_is_required() {
        insta::assert_debug_snapshot!(docs_error_kind(&[]), @"MissingRequiredArgument");
    }

    #[test]
    fn typescript_has_default_output() {
        let options = typescript(&["typescript"]);
        insta::assert_debug_snapshot!(options.output, @r#""src-generated""#);
    }
}
