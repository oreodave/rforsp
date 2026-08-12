//! Entrypoint for rForsp compiler

use std::{io::Write, process::ExitCode};

use rforsp::{
    diagnostics::{Diagnostics, render_diagnostics},
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

/// Report errors from [`Diagnostics`] to [`std::io::stderr`] if there are any.
fn report_errors(table: &SourceTable, diagnostics: &Diagnostics) -> bool {
    if diagnostics.has_errors() {
        let mut error_buf = String::new();
        let _ = render_diagnostics(diagnostics, table, &mut error_buf);
        let _ = std::io::stderr().write_all(error_buf.as_bytes());
        true
    } else {
        false
    }
}

fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        usage(std::io::stderr());
        return ExitCode::FAILURE;
    }

    let mut diagnostics = Diagnostics::new();
    let mut table = SourceTable::new();
    let sources = args
        .iter()
        .filter_map(|filename| {
            table
                .add_source_file(filename)
                .map_err(|e| {
                    diagnostics.push(e.into());
                })
                .ok()
        })
        .collect::<Vec<_>>();

    if diagnostics.has_errors() {
        // FIXME: Make this phase generic
        eprintln!("Compilation failed during Source phase.");
        report_errors(&table, &diagnostics);
        ExitCode::FAILURE
    } else {
        for source in sources.iter().map(|&id| table.get_source(id)) {
            println!(
                concat!("SOURCE[{}]:\n", "<start>\n", "{}", "<end>\n"),
                source.name,
                source.text()
            );
        }

        ExitCode::SUCCESS
    }
}
