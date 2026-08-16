//! Entrypoint for rForsp compiler

use std::process::ExitCode;

use rforsp::{
    context::Compilation,
    diagnostics::{Diagnostics, render_diagnostics},
    drivers::compile,
    log::Log,
};

/// Configuration for CLI driver.
struct CliConfig {
    /// Log level.
    log: Log,
    /// Files to compile.
    files: Vec<String>,
}

/// Types of immediate CLI exit following command line parsing.
enum CliExit {
    /// Immediately exit with failure.
    Failure,
    /// Immediately exit with success.
    Success,
}

/// Parse command line arguments
fn parse_cli() -> Result<CliConfig, CliExit> {
    let mut config = CliConfig {
        log: Log::NONE,
        files: Vec::new(),
    };
    let mut args = std::env::args().skip(1).peekable();

    while let Some(arg) = args.peek()
        && arg.starts_with("--")
    {
        match args.next().unwrap_or_default().as_str() {
            "--log-tokens" => config.log.insert(Log::TOKENS),
            "--log-hir" => config.log.insert(Log::HIR),
            "--help" => {
                usage(std::io::stdout());
                return Err(CliExit::Success);
            }
            "--version" => {
                println!("rforsp v0.0.0");
                return Err(CliExit::Success);
            }
            unknown => {
                eprintln!("Unknown argument `{unknown}`.");
                return Err(CliExit::Failure);
            }
        }
    }

    if args.len() == 0 {
        Err(CliExit::Failure)
    } else {
        config.files = args.collect();
        Ok(config)
    }
}

/// Print usage to the given [`Write`] target.
fn usage(mut out: impl std::io::Write) {
    let _ = write!(
        out,
        concat!(
            "Usage: rforsp [OPTIONS] [FILES]\n",
            "Compile the given FILES sequentially as rforsp source code.\n",
            "Options:\n",
            "  --help:       Print this help and exit.\n",
            "  --version:    Print version of program.\n",
            "  --log-tokens: Print tokens generated per FILE.\n",
            "  --log-hir:    Print AST generated over all FILES.\n",
        )
    );
}

fn main() -> ExitCode {
    let config = match parse_cli() {
        Err(CliExit::Failure) => {
            usage(std::io::stderr());
            return ExitCode::FAILURE;
        }
        Err(CliExit::Success) => {
            return ExitCode::SUCCESS;
        }
        Ok(cfg) => cfg,
    };

    let args = config.files;

    let mut ctx = Compilation::new();
    let mut diagnostics = Diagnostics::new();

    let mut log_buf = String::new();
    let compile_result =
        compile(&args, &mut ctx, &mut diagnostics, config.log, &mut log_buf);

    print!("{log_buf}");

    match compile_result {
        Err(e) => {
            eprintln!("{e}");
            let mut error_buf = String::new();
            let _ =
                render_diagnostics(&diagnostics, &ctx.table, &mut error_buf);
            eprint!("{error_buf}");
            ExitCode::FAILURE
        }
        Ok(()) => ExitCode::SUCCESS,
    }
}
