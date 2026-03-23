use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    let mut script_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    script_path.pop();
    script_path.push("publish.sh");

    let mut command = Command::new("bash");
    command.arg(&script_path);
    command.args(env::args_os().skip(1));

    match command.status() {
        Ok(status) => {
            std::process::exit(status.code().unwrap_or(1));
        }
        Err(error) => {
            eprintln!(
                "[error] failed to execute {}: {error}",
                script_path.display()
            );
            std::process::exit(1);
        }
    }
}
