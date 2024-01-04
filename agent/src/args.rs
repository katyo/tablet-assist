use argp::FromArgs;
use std::path::PathBuf;
#[cfg(feature = "tracing-subscriber")]
use tracing_subscriber::EnvFilter;

/// Tablet-mode assistance DBus session service.
#[derive(FromArgs, Debug)]
pub struct Args {
    /// Path to config file.
    #[argp(
        option,
        short = 'c',
        arg_name = "path",
        default = "Args::default_config()"
    )]
    pub config: PathBuf,

    /// Log/trace filter.
    #[cfg(feature = "tracing-subscriber")]
    #[argp(
        option,
        short = 't',
        arg_name = "filter",
        from_str_fn(Args::parse_env_filter)
    )]
    pub trace: Option<EnvFilter>,

    /// Log to stdout.
    #[cfg(feature = "stderr")]
    #[argp(switch, short = 'l')]
    pub log: bool,

    /// Log to journald.
    #[cfg(feature = "journal")]
    #[argp(switch, short = 'j')]
    pub journal: bool,

    /// Show version and exit.
    #[argp(switch, short = 'v')]
    pub version: bool,
}

impl Args {
    /// Create args from command-line
    pub fn new() -> Self {
        argp::parse_args_or_exit(argp::DEFAULT)
    }

    fn default_config() -> PathBuf {
        let prefix = dirs::config_dir().unwrap();
        prefix.join("tablet-assist").join("config.toml")
    }

    #[cfg(feature = "tracing-subscriber")]
    fn parse_env_filter(s: &str) -> Result<EnvFilter, String> {
        s.parse()
            .map_err(|error| format!("Bad tracing filter: {error}"))
    }
}
