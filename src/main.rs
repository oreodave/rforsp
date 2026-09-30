//! Entrypoint for rForsp compiler

use std::process::ExitCode;

use rforsp::{
    context::Compilation,
    diagnostics::{Diagnostics, render_diagnostics},
    drivers::{SourceInput, compile},
    log::Log,
};

/// Configuration for CLI driver.
struct CliConfig {
    /// Stages to log.
    log: Log,
    /// Source inputs to compile, in the order given.
    inputs: Vec<SourceInput>,
}

/// Immediate exit requested while parsing arguments.
enum CliExit {
    /// Exit 1.  The caller prints usage.
    Failure,
    /// Exit 0.  The output is already printed.
    Success,
}

/// Parse command line arguments.
///
/// Options precede files.  Parsing stops at the first argument that does not
/// begin with `--`.
fn parse_cli() -> Result<CliConfig, CliExit> {
    let mut config = CliConfig {
        log: Log::NONE,
        inputs: Vec::new(),
    };
    let mut args = std::env::args().skip(1).peekable();
    let mut std_included = false;

    while let Some(arg) = args.peek()
        && arg.starts_with("--")
    {
        match args.next().unwrap_or_default().as_str() {
            "--log-tokens" => config.log.insert(Log::TOKENS),
            "--log-hir" => config.log.insert(Log::HIR),
            "--log-resolution" => config.log.insert(Log::RESOLUTION),
            "--std" if !std_included => {
                std_included = true;
                config.inputs.push(SourceInput::Embedded {
                    name: concat!(env!("CARGO_MANIFEST_DIR"), "/lib/std.rfp"),
                    contents: include_str!("../lib/std.rfp"),
                });
            }
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
        config.inputs.extend(args.map(SourceInput::File));
        Ok(config)
    }
}

/// Print usage to the given `out` target.
fn usage(mut out: impl std::io::Write) {
    // Every caller exits immediately after this, so a failed write to the
    // terminal has nowhere left to be reported.
    let _ = write!(
        out,
        concat!(
            "Usage: rforsp [OPTIONS] [FILES]\n",
            "Compile the given FILES sequentially as rforsp source code.\n",
            "Options:\n",
            "  --help:            Print this help and exit.\n",
            "  --version:         Print version of program.\n",
            "  --log-tokens:      Print tokens generated per FILE.\n",
            "  --log-hir:         Print AST generated over all FILES.\n",
            "  --log-resolution:  Print resolutions derived from parse tree.\n",
            "  --std:             Include the standard library during compilation.\n",
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

    let inputs = config.inputs;
    let mut ctx = Compilation::new();
    let mut diagnostics = Diagnostics::new();

    let mut log_buf = String::new();
    let compile_result = compile(
        &inputs,
        &mut ctx,
        &mut diagnostics,
        config.log,
        &mut log_buf,
    );

    print!("{log_buf}");

    match compile_result {
        Err(e) => {
            eprintln!("{e}");
            let mut error_buf = String::new();
            // The target is a String, so the only error this can carry is one
            // the renderer invents, which it does not.
            let _ =
                render_diagnostics(&diagnostics, &ctx.table, &mut error_buf);
            eprint!("{error_buf}");
            ExitCode::FAILURE
        }
        Ok(()) => ExitCode::SUCCESS,
    }
}
