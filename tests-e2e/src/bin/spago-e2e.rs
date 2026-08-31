use std::env;
use std::fs::OpenOptions;
use std::io::Write;
use std::process::{self, Command};

fn main() {
    let arguments = env::args_os().skip(1).collect::<Vec<_>>();
    let log_path = env::var_os("ALEXANDRITE_E2E_SPAGO_LOG").expect("missing Spago log path");
    let current_directory = env::current_dir().expect("failed to read Spago working directory");
    let mut log = OpenOptions::new().create(true).append(true).open(log_path).unwrap();
    let mut record = current_directory.display().to_string();
    for argument in &arguments {
        record.push('\t');
        record.push_str(&argument.to_string_lossy());
    }
    record.push('\n');
    log.write_all(record.as_bytes()).unwrap();

    let executable = env::var_os("ALEXANDRITE_E2E_SPAGO").expect("missing real Spago executable");
    let status = Command::new(executable).args(arguments).status().unwrap();
    process::exit(status.code().unwrap_or(1));
}
