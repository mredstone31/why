use clap::{Parser, ValueHint};

#[derive(Debug, Parser)]
#[command(
    name = "why",
    version,
    about = "Explain why a Linux command is slow, waiting, or file-heavy",
    long_about = None
)]
pub struct Cli {
    #[arg(
        short,
        long,
        default_value_t = 15,
        help = "Stop tracing after this many seconds"
    )]
    pub timeout: u64,

    #[arg(
        long,
        help = "Show the result in a fullscreen TUI dashboard"
    )]
    pub tui: bool,

    #[arg(
        long,
        help = "Keep the raw trace file instead of deleting it automatically"
    )]
    pub keep_trace: bool,

    #[arg(
        long,
        help = "Disable colored output"
    )]
    pub no_color: bool,

    #[arg(
        required = true,
        num_args = 1..,
        trailing_var_arg = true,
        allow_hyphen_values = true,
        value_hint = ValueHint::CommandWithArguments,
        help = "Command to run and analyze"
    )]
    pub command: Vec<String>,
}

impl Cli {
    pub fn program(&self) -> &str {
        &self.command[0]
    }

    pub fn program_args(&self) -> &[String] {
        &self.command[1..]
    }
}