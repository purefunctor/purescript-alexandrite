use std::path::PathBuf;

use clap::error::ErrorKind;
use clap::{Arg, ArgAction, ArgMatches, Args, FromArgMatches, Parser, Subcommand};
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
    #[command(flatten)]
    pub packages: PackageSpecs,
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

#[derive(Debug, Clone)]
pub struct PackageSpec {
    pub name: String,
    pub version: String,
    pub sources: Vec<String>,
}

impl PackageSpec {
    pub fn parse(arguments: impl Iterator<Item = String>) -> Result<PackageSpec, String> {
        let mut arguments = arguments.into_iter();

        let name_and_version = arguments.next().expect("clap enforces num_args(2..)");
        let (name, version) = name_and_version.split_once('@').ok_or_else(|| {
            format!("package specifier `{name_and_version}` must be NAME@VERSION")
        })?;

        if name.is_empty() {
            return Err(format!("package specifier `{name_and_version}` has an empty name"));
        }

        if version.is_empty() {
            return Err(format!("package specifier `{name_and_version}` has an empty version"));
        }

        let sources = arguments.collect();
        Ok(PackageSpec { name: name.to_owned(), version: version.to_owned(), sources })
    }
}

#[derive(Debug)]
pub struct PackageSpecs {
    pub packages: Vec<PackageSpec>,
}

impl Args for PackageSpecs {
    fn augment_args(cmd: clap::Command) -> clap::Command {
        cmd.arg(
            Arg::new("package")
                .long("package")
                .value_names(["NAME@VERSION", "SOURCES"])
                .num_args(2..)
                .action(ArgAction::Append)
                .required_unless_present("spago_project")
                .value_parser(clap::value_parser!(String))
                .help("A package to document"),
        )
    }

    fn augment_args_for_update(cmd: clap::Command) -> clap::Command {
        PackageSpecs::augment_args(cmd)
    }
}

impl FromArgMatches for PackageSpecs {
    fn from_arg_matches(matches: &ArgMatches) -> Result<PackageSpecs, clap::Error> {
        let mut matches = matches.clone();
        PackageSpecs::from_arg_matches_mut(&mut matches)
    }

    fn from_arg_matches_mut(matches: &mut ArgMatches) -> Result<PackageSpecs, clap::Error> {
        let Some(occurrences) = matches.remove_occurrences::<String>("package") else {
            return Ok(PackageSpecs { packages: vec![] });
        };
        let packages = occurrences
            .map(PackageSpec::parse)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|message| clap::Error::raw(ErrorKind::ValueValidation, message))?;
        Ok(PackageSpecs { packages })
    }

    fn update_from_arg_matches(&mut self, matches: &ArgMatches) -> Result<(), clap::Error> {
        let mut matches = matches.clone();
        self.update_from_arg_matches_mut(&mut matches)
    }

    fn update_from_arg_matches_mut(&mut self, matches: &mut ArgMatches) -> Result<(), clap::Error> {
        if let Some(occurrences) = matches.remove_occurrences::<String>("package") {
            let packages = occurrences
                .map(PackageSpec::parse)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|message| clap::Error::raw(ErrorKind::ValueValidation, message))?;
            self.packages.extend(packages);
        }
        Ok(())
    }
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
    fn single_package_with_one_source() {
        let options = docs(&["--package", "effect@v0.0.0", "src/**/*.purs"]);
        insta::assert_debug_snapshot!(options.packages, @r#"
        PackageSpecs {
            packages: [
                PackageSpec {
                    name: "effect",
                    version: "v0.0.0",
                    sources: [
                        "src/**/*.purs",
                    ],
                },
            ],
        }
        "#);
    }

    #[test]
    fn single_package_with_multiple_sources() {
        let options = docs(&["--package", "effect@v0.0.0", "src/**/*.purs", "test/**/*.purs"]);
        insta::assert_debug_snapshot!(options.packages, @r#"
        PackageSpecs {
            packages: [
                PackageSpec {
                    name: "effect",
                    version: "v0.0.0",
                    sources: [
                        "src/**/*.purs",
                        "test/**/*.purs",
                    ],
                },
            ],
        }
        "#);
    }

    #[test]
    fn repeated_packages_keep_occurrence_grouping() {
        let options = docs(&[
            "--package",
            "effect@v0.0.0",
            "src/**/*.purs",
            "--package",
            "prelude@v1.2.3",
            "src/**/*.purs",
            "test/**/*.purs",
        ]);
        insta::assert_debug_snapshot!(options.packages, @r#"
        PackageSpecs {
            packages: [
                PackageSpec {
                    name: "effect",
                    version: "v0.0.0",
                    sources: [
                        "src/**/*.purs",
                    ],
                },
                PackageSpec {
                    name: "prelude",
                    version: "v1.2.3",
                    sources: [
                        "src/**/*.purs",
                        "test/**/*.purs",
                    ],
                },
            ],
        }
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
            PackageSpecs {
                packages: [],
            },
        )
        "#);
    }

    #[test]
    fn spago_project_conflicts_with_package_specs() {
        insta::assert_debug_snapshot!(
            docs_error_kind(&["--spago-project", ".", "--package", "effect@v0.0.0", "src/**/*.purs"]),
            @"ArgumentConflict"
        );
    }

    #[test]
    fn later_flags_are_not_consumed_as_package_sources() {
        let options = docs(&["--package", "effect@v0.0.0", "src/**/*.purs", "--output", "out"]);
        insta::assert_debug_snapshot!((&options.output, &options.packages), @r#"
        (
            "out",
            PackageSpecs {
                packages: [
                    PackageSpec {
                        name: "effect",
                        version: "v0.0.0",
                        sources: [
                            "src/**/*.purs",
                        ],
                    },
                ],
            },
        )
        "#);
    }

    #[test]
    fn missing_source_is_rejected() {
        insta::assert_debug_snapshot!(docs_error_kind(&["--package", "effect@v0.0.0"]), @"TooFewValues");
    }

    #[test]
    fn missing_version_separator_is_rejected() {
        insta::assert_debug_snapshot!(
            docs_error_kind(&["--package", "effect", "src/**/*.purs"]),
            @"ValueValidation"
        );
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
