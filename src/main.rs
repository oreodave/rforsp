//! Entrypoint for rForsp compiler

use std::{io::Write, process::ExitCode};

use rforsp::{
    diagnostics::{Aborted, Diagnostics, render_diagnostics},
    drivers::{lex_sources, sources_from_files},
    source::SourceTable,
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

/// Compile a set of `filenames`.
fn compile(
    filenames: &[String],
    table: &mut SourceTable,
    diagnostics: &mut Diagnostics,
) -> Result<(), Aborted> {
    let sources = sources_from_files(filenames, table, diagnostics)?;
    let lexes = lex_sources(&sources, table, diagnostics)?;

    for (&id, lex_stream) in sources.iter().zip(lexes) {
        let source = table.get_source(id);
        println!(
            "{}: {} bytes => {} tokens",
            source.name,
            source.len(),
            lex_stream.len()
        );
        for token in &lex_stream {
            let kind = token.kind;
            let text = source.span_text(token.span);
            print!("{kind:?}({text}), ");
        }
        println!();
    }

    Ok(())
}

fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        usage(std::io::stderr());
        return ExitCode::FAILURE;
    }

    let mut diagnostics = Diagnostics::new();
    let mut table = SourceTable::new();

    let compile_result = compile(&args, &mut table, &mut diagnostics);

    match compile_result {
        Err(Aborted(phase)) => {
            eprintln!("Compilation failed during {} phase", phase.as_str());
            let mut error_buf = String::new();
            let _ = render_diagnostics(&diagnostics, &table, &mut error_buf);
            let _ = std::io::stderr().write_all(error_buf.as_bytes());
            ExitCode::FAILURE
        }
        Ok(()) => ExitCode::SUCCESS,
    }
}
