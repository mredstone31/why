mod cli;
mod trace;
mod classify;
mod insights;
mod report;
mod tui;

use clap::Parser;
use cli::Cli;
use colored::control;
use colored::Colorize;
use report::{analyze_trace, print_report};
use trace::run_trace;
use tui::run_tui;

fn main() {
    let cli = Cli::parse();

    if cli.no_color {
        control::set_override(false);
    } else {
        control::set_override(true);
    }

    match run_trace(
        cli.program(),
        cli.program_args(),
        cli.timeout,
        cli.keep_trace,
    ) {
        Ok(result) => {
            let command_text = if cli.program_args().is_empty() {
                cli.program().to_string()
            } else {
                format!("{} {}", cli.program(), cli.program_args().join(" "))
            };

            let analysis = analyze_trace(&result.trace_contents);

            if cli.tui {
                if let Err(err) = run_tui(
                    &command_text,
                    result.exit_code,
                    result.wall_time,
                    result.kept_trace_path.as_deref(),
                    result.timed_out,
                    cli.timeout,
                    &analysis,
                    &result.trace_contents,
                    cli.no_color,
                ) {
                    eprintln!("{}", format!("Error: {}", err).red().bold());
                    std::process::exit(1);
                }
            } else {
                print_report(
                    &command_text,
                    result.exit_code,
                    result.wall_time,
                    result.kept_trace_path.as_deref(),
                    result.timed_out,
                    cli.timeout,
                    &analysis,
                );
            }
        }
        Err(err) => {
            eprintln!("{}", format!("Error: {}", err).red().bold());
            std::process::exit(1);
        }
    }
}