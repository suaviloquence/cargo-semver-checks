#![forbid(unsafe_code)]
// #![cfg(not(test))]

use std::env;

mod cli;

#[cfg(test)]
mod snapshot_tests_copy;

use cargo_semver_checks::{GlobalConfig, SemverQuery};
use clap::Parser;

use cli::{Cargo, SemverChecksCommands};

fn main() {
    human_panic::setup_panic!();

    let Cargo::SemverChecks(args) = Cargo::parse();

    configure_color(args.color_choice);

    if args.bugreport {
        use bugreport::{bugreport, collector::*, format::Markdown};
        bugreport!()
            .info(SoftwareVersion::default())
            .info(OperatingSystem::default())
            .info(CommandLine::default())
            .info(CommandOutput::new("cargo version", "cargo", &["-V"]))
            .info(CompileTimeInformation::default())
            .print::<Markdown>();
        std::process::exit(0);
    } else if args.list {
        exit_on_error(true, || {
            let mut config = GlobalConfig::new();
            config.set_log_level(args.check_release.verbosity.log_level());

            let queries = SemverQuery::all_queries();
            let mut rows = vec![["id", "type", "description"], ["==", "====", "==========="]];
            for query in queries.values() {
                rows.push([
                    query.id.as_str(),
                    query.required_update.as_str(),
                    query.description.as_str(),
                ]);
            }
            let mut widths = [0; 3];
            for row in &rows {
                widths[0] = widths[0].max(row[0].len());
                widths[1] = widths[1].max(row[1].len());
                widths[2] = widths[2].max(row[2].len());
            }
            let stdout = std::io::stdout();
            let mut stdout = stdout.lock();
            for row in rows {
                use std::io::Write;
                writeln!(
                    stdout,
                    "{0:<1$} {2:<3$} {4:<5$}",
                    row[0], widths[0], row[1], widths[1], row[2], widths[2]
                )?;
            }

            config.shell_note("Use `--explain <id>` to see more details")
        });
        std::process::exit(0);
    } else if let Some(id) = args.explain.as_deref() {
        exit_on_error(true, || {
            let queries = SemverQuery::all_queries();
            let query = queries.get(id).ok_or_else(|| {
                let ids = queries.keys().cloned().collect::<Vec<_>>();
                anyhow::format_err!(
                    "Unknown id `{}`, available id's:\n  {}",
                    id,
                    ids.join("\n  ")
                )
            })?;
            println!(
                "{}",
                query
                    .reference
                    .as_deref()
                    .unwrap_or(query.description.as_str())
            );
            if let Some(link) = &query.reference_link {
                println!();
                println!("See also {link}");
            }
            Ok(())
        });
        std::process::exit(0);
    }

    let check_release = match args.command {
        Some(SemverChecksCommands::CheckRelease(c)) => c,
        None => args.check_release,
    };

    let mut config = GlobalConfig::new();

    config.set_log_level(check_release.verbosity.log_level());

    let check: cargo_semver_checks::Check = check_release.into();

    let report = exit_on_error(config.is_error(), || check.check_release(&mut config));
    if report.success() {
        std::process::exit(0);
    } else {
        std::process::exit(1);
    }
}

fn exit_on_error<T>(log_errors: bool, mut inner: impl FnMut() -> anyhow::Result<T>) -> T {
    match inner() {
        Ok(x) => x,
        Err(err) => {
            if log_errors {
                eprintln!("error: {err:?}");
            }
            std::process::exit(1)
        }
    }
}

/// helper function to determine whether to use colors based on the (passed) `--color` flag
/// and the value of the `CARGO_TERM_COLOR` variable.
///
/// If the `--color` flag is set to something valid, it overrides anything in
/// the `CARGO_TERM_COLOR` environment variable
fn configure_color(cli_choice: Option<clap::ColorChoice>) {
    use anstream::ColorChoice as AnstreamChoice;
    use clap::ColorChoice as ClapChoice;
    let choice = match cli_choice {
        Some(ClapChoice::Always) => AnstreamChoice::Always,
        Some(ClapChoice::Auto) => AnstreamChoice::Auto,
        Some(ClapChoice::Never) => AnstreamChoice::Never,
        // we match the behavior of cargo in
        // https://doc.rust-lang.org/cargo/reference/config.html#termcolor
        // note that [`ColorChoice::AlwaysAnsi`] is not supported by cargo.
        None => match env::var("CARGO_TERM_COLOR").as_deref() {
            Ok("always") => AnstreamChoice::Always,
            Ok("never") => AnstreamChoice::Never,
            // if `auto` is set, or the env var is invalid
            // or both the env var and flag are not set, we set the choice to auto
            _ => AnstreamChoice::Auto,
        },
    };

    choice.write_global();
}
