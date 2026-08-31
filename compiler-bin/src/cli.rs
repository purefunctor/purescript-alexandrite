use std::borrow::Cow;
use std::io;
use std::path::PathBuf;

use clap::builder::{PathBufValueParser, TypedValueParser};
use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};
use path_absolutize::Absolutize;
use tracing::level_filters::LevelFilter;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn absolute_path(value: PathBuf) -> io::Result<PathBuf> {
    value.absolutize().map(Cow::into_owned)
}

fn absolute_path_parser() -> impl TypedValueParser<Value = PathBuf> {
    PathBufValueParser::new().try_map(absolute_path)
}

#[derive(Debug, Parser)]
#[command(about, version(VERSION))]
pub struct Cli {
    /// Print log path.
    #[arg(long)]
    pub log_file: bool,
    #[command(flatten)]
    pub lsp: LspOptions,
    #[command(subcommand)]
    pub command: Option<Command>,
}

impl Cli {
    pub fn command(self) -> Command {
        self.command.unwrap_or(Command::Lsp(self.lsp))
    }
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Run the language server.
    Lsp(LspOptions),
    /// Create a Spago project in the current directory.
    New(NewOptions),
    /// Build a Spago workspace or package.
    Build(ProjectBuildOptions),
    /// Add dependencies to a Spago package.
    Add(AddOptions),
    /// Build and run a Spago package with Node.js.
    Run(RunOptions),
    /// Build and test one or more Spago packages with Node.js.
    Test(TestOptions),
    /// Compile PureScript modules to JavaScript (experimental).
    Compile(CompileOptions),
    /// Build a Spago workspace or package and rebuild when inputs change.
    Watch(ProjectBuildOptions),
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
pub struct CompileOptions {
    #[command(flatten)]
    pub build: BuildOptions,

    /// Package folder to compile.
    #[arg(
        id = "package",
        long = "package",
        value_name("DIR"),
        value_parser = absolute_path_parser()
    )]
    pub packages: Vec<PathBuf>,

    /// PureScript source paths or glob patterns.
    #[arg(value_name("INPUT"), required_unless_present("package"))]
    pub inputs: Vec<PathBuf>,

    /// Code generation targets requested by build tools.
    #[arg(long, value_name("TARGETS"))]
    pub codegen: Option<String>,

    /// Emit a Spago-compatible JSON result.
    ///
    /// Full structured JSON diagnostics are not yet supported.
    #[arg(long)]
    pub json_errors: bool,
}

#[derive(Debug, Args)]
pub struct BuildOptions {
    #[command(flatten)]
    pub logging: LoggingOptions,

    /// Output directory for compiled modules.
    #[arg(short, long, value_name("DIR"), default_value("output"), value_parser = absolute_path_parser())]
    pub output: PathBuf,

    /// Suppress build progress output.
    #[arg(short, long)]
    pub quiet: bool,

    /// When to use colors in human-readable diagnostics.
    #[arg(long, value_enum, default_value_t = ColorChoice::Auto)]
    pub color: ColorChoice,
}

#[derive(Debug, Args)]
pub struct NewOptions {
    /// Package name. Defaults to the current directory name.
    #[arg(long, value_name("NAME"))]
    pub name: Option<String>,
}

#[derive(Debug, Args)]
pub struct ProjectBuildOptions {
    #[command(flatten)]
    pub logging: LoggingOptions,

    /// Workspace package to build.
    #[arg(short, long, value_name("NAME"))]
    pub package: Option<String>,

    /// Output directory for compiled modules. Defaults to output in the workspace root.
    #[arg(short, long, value_name("DIR"), value_parser = absolute_path_parser())]
    pub output: Option<PathBuf>,

    /// Suppress build progress output.
    #[arg(short, long)]
    pub quiet: bool,

    /// When to use colors in human-readable diagnostics.
    #[arg(long, value_enum, default_value_t = ColorChoice::Auto)]
    pub color: ColorChoice,
}

#[derive(Debug, Args)]
pub struct AddOptions {
    /// Workspace package whose dependencies should change.
    #[arg(short, long, value_name("NAME"))]
    pub package: Option<String>,

    /// Add packages as test dependencies.
    #[arg(long)]
    pub test: bool,

    /// Packages to add.
    #[arg(value_name("DEPENDENCY"), required = true)]
    pub dependencies: Vec<String>,
}

#[derive(Debug, Args)]
pub struct RunOptions {
    #[command(flatten)]
    pub build: ProjectBuildOptions,

    /// Module containing the program entry point.
    #[arg(long, value_name("MODULE"))]
    pub main: Option<String>,

    /// Arguments passed to the program.
    #[arg(last = true)]
    pub arguments: Vec<String>,
}

#[derive(Debug, Args)]
pub struct TestOptions {
    #[command(flatten)]
    pub build: ProjectBuildOptions,

    /// Module containing the test entry point.
    #[arg(long, value_name("MODULE"))]
    pub main: Option<String>,

    /// Arguments passed to each test program.
    #[arg(last = true)]
    pub arguments: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ColorChoice {
    Auto,
    Always,
    Never,
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
    #[arg(long, value_name("DIR"), default_value("docs"), value_parser = absolute_path_parser())]
    pub output: PathBuf,
    /// Suppress documentation progress output.
    #[arg(short, long)]
    pub quiet: bool,
    /// Spago project directory containing spago.lock.
    #[arg(long, value_name("DIR"), conflicts_with("package"), value_parser = absolute_path_parser())]
    pub spago_project: Option<PathBuf>,
    /// Package folder to document.
    #[arg(
        id = "package",
        long = "package",
        value_name("DIR"),
        value_parser = absolute_path_parser(),
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
    #[arg(long, value_name("DIR"), default_value("src-generated"), value_parser = absolute_path_parser())]
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
    use std::path::Path;

    fn lsp(args: &[&str]) -> LspOptions {
        let mut argv = vec!["alexandrite"];
        argv.extend(args);
        let cli = Cli::parse_from(argv);
        match cli.command() {
            Command::Lsp(options) => options,
            _ => unreachable!("parsed command was not `lsp`"),
        }
    }

    fn docs(args: &[&str]) -> DocsOptions {
        let mut argv = vec!["alexandrite", "docs"];
        argv.extend(args);
        let cli = Cli::parse_from(argv);
        match cli.command {
            Some(Command::Docs(options)) => options,
            _ => unreachable!("parsed command was not `docs`"),
        }
    }

    fn compile(args: &[&str]) -> CompileOptions {
        let mut argv = vec!["alexandrite", "compile"];
        argv.extend(args);
        let cli = Cli::parse_from(argv);
        match cli.command {
            Some(Command::Compile(options)) => options,
            _ => unreachable!("parsed command was not `compile`"),
        }
    }

    fn compile_error_kind(args: &[&str]) -> ErrorKind {
        let mut argv = vec!["alexandrite", "compile"];
        argv.extend(args);
        Cli::try_parse_from(argv).unwrap_err().kind()
    }

    fn watch(args: &[&str]) -> ProjectBuildOptions {
        let mut argv = vec!["alexandrite", "watch"];
        argv.extend(args);
        let cli = Cli::parse_from(argv);
        match cli.command {
            Some(Command::Watch(options)) => options,
            _ => unreachable!("parsed command was not `watch`"),
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

    fn current_directory() -> PathBuf {
        std::env::current_dir().unwrap()
    }

    fn current_directory_path(path: impl AsRef<Path>) -> PathBuf {
        current_directory().join(path)
    }

    #[test]
    fn lsp_is_the_default_command() {
        let options = lsp(&["--stdio", "--diagnostics-on-change"]);

        assert!(options.stdio);
        assert!(options.diagnostics_on_change);
    }

    #[test]
    fn compile_accepts_spago_arguments() {
        let options = compile(&[
            "--codegen",
            "corefn,docs,js,sourcemaps",
            "--json-errors",
            "--quiet",
            "--color",
            "always",
            "src/**/*.purs",
            ".spago/p/prelude-6.0.2/src/**/*.purs",
        ]);

        assert_eq!(options.build.output, current_directory_path("output"));
        assert_eq!(options.codegen.as_deref(), Some("corefn,docs,js,sourcemaps"));
        assert!(options.json_errors);
        assert!(options.build.quiet);
        assert_eq!(options.build.color, ColorChoice::Always);
        assert_eq!(
            options.inputs,
            vec![
                PathBuf::from("src/**/*.purs"),
                PathBuf::from(".spago/p/prelude-6.0.2/src/**/*.purs"),
            ]
        );
    }

    #[test]
    fn watch_accepts_project_build_arguments() {
        let options =
            watch(&["--package", "application", "--output", "dist", "--quiet", "--color", "never"]);

        assert_eq!(options.package.as_deref(), Some("application"));
        assert_eq!(options.output, Some(current_directory_path("dist")));
        assert!(options.quiet);
        assert_eq!(options.color, ColorChoice::Never);
    }

    #[test]
    fn compile_accepts_package_folders_without_inputs() {
        let options =
            compile(&["--package", "packages/effect", "--package", "packages/prelude", "--quiet"]);

        assert_eq!(
            options.packages,
            vec![
                current_directory_path("packages/effect"),
                current_directory_path("packages/prelude"),
            ]
        );
        assert!(options.inputs.is_empty());
        assert!(options.build.quiet);
    }

    #[test]
    fn compile_requires_inputs_or_a_package() {
        insta::assert_debug_snapshot!(compile_error_kind(&[]), @"MissingRequiredArgument");
    }

    #[test]
    fn single_package_folder() {
        let options = docs(&["--package", "packages/effect", "--quiet"]);

        assert_eq!(options.packages, vec![current_directory_path("packages/effect")]);
        assert!(options.quiet);
    }

    #[test]
    fn repeated_packages_keep_order() {
        let options = docs(&["--package", "packages/effect", "--package", "packages/prelude"]);

        assert_eq!(
            options.packages,
            vec![
                current_directory_path("packages/effect"),
                current_directory_path("packages/prelude"),
            ]
        );
    }

    #[test]
    fn spago_project_replaces_package_specs() {
        let options = docs(&["--spago-project", "."]);

        assert_eq!(options.spago_project, Some(current_directory()));
        assert!(options.packages.is_empty());
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

        assert_eq!(options.output, current_directory_path("out"));
        assert_eq!(options.packages, vec![current_directory_path("packages/effect")]);
    }

    #[test]
    fn relative_paths_are_normalised() {
        let options = docs(&["--spago-project", "./spago/..", "--output", "./generated/../docs"]);

        assert_eq!(options.spago_project, Some(current_directory()));
        assert_eq!(options.output, current_directory_path("docs"));
    }

    #[test]
    fn absolute_paths_are_not_resolved_from_the_current_directory() {
        let output = std::env::temp_dir().join("alexandrite-docs-output");
        let options = docs(&["--package", "packages/effect", "--output", output.to_str().unwrap()]);

        assert_eq!(options.output, output);
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

        assert_eq!(options.output, current_directory_path("src-generated"));
    }
}
