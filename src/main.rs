#![expect(clippy::print_stderr, reason = "CLI output")]
use otterlang::cli;

fn main() {
    if let Err(e) = cli::run() {
        let msg = e.to_string();

        // Known compilation failures already emitted diagnostics
        if msg.contains("lexing failed")
            || msg.contains("parsing failed")
            || msg.contains("type checking failed")
        {
            std::process::exit(1);
        }

        eprintln!("Error: {msg}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    #![expect(clippy::panic, reason = "Tests can panic to fail")]

    use clap::Parser;
    use otterlang::cli::{Command, OtterCli};
    use std::path::Path;

    #[test]
    fn build_command_honors_output_flag() {
        let cli = OtterCli::parse_from([
            "otter",
            "build",
            "examples/app.ot",
            "--output",
            "target/app",
        ]);

        match cli.command() {
            Command::Build { path, output } => {
                assert_eq!(path, Path::new("examples/app.ot"));
                assert_eq!(output.as_deref(), Some(Path::new("target/app")));
            }
            other => panic!("expected build command, got {other:?}"),
        }
    }
}
