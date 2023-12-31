use argp::FromArgs;
use std::path::PathBuf;

/// Tablet mode detection service
#[derive(FromArgs, Debug)]
pub struct Args {
    /// Path to config file
    #[argp(option, short = 'c')]
    pub config: Option<PathBuf>,

    /// Run dbus service
    #[argp(switch, short = 'd')]
    pub dbus: bool,
}

impl Args {
    /// Create args from command-line
    pub fn new() -> Self {
        argp::parse_args_or_exit(argp::DEFAULT)
    }
}
