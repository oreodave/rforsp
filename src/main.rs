//! Entrypoint for rForsp compiler

use std::process::ExitCode;

use rforsp::{
    context::Compilation,
    diagnostics::{Diagnostics, render_diagnostics},
    drivers::compile,
};

/// Print usage to the given [`Write`] target.
fn usage(mut out: impl std::io::Write) {
    let _ = writeln!(
        out,
        concat!(
            "Usage: rforsp [FILES]\n",
            "Compile the given FILES sequentially as rforsp source code."
        )
    );
}

fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        usage(std::io::stderr());
        return ExitCode::FAILURE;
    }

    let mut ctx = Compilation::new();
    let mut diagnostics = Diagnostics::new();

    let compile_result = compile(&args, &mut ctx, &mut diagnostics);

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
