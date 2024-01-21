use argp::FromArgs;
#[cfg(feature = "tracing-subscriber")]
use tracing_subscriber::EnvFilter;

/// Tablet-mode assistance UI.
#[derive(FromArgs, Debug)]
pub struct Args {
    /// Locale to use.
    #[argp(
        option,
        short = 'L',
        arg_name = "code",
        default = "Args::default_locale()"
    )]
    pub locale: String,

    /// Logging filter.
    #[cfg(feature = "tracing-subscriber")]
    #[argp(
        option,
        short = 'l',
        arg_name = "filter",
        from_str_fn(Args::parse_env_filter)
    )]
    pub log: Option<EnvFilter>,

    /// Show version and exit.
    #[argp(switch, short = 'v')]
    pub version: bool,
}

impl Args {
    /// Create args from command-line
    pub fn new() -> Self {
        argp::parse_args_or_exit(argp::DEFAULT)
    }

    fn default_locale() -> String {
        sys_locale::get_locale().unwrap_or_else(|| "en-US".into())
    }

    #[cfg(feature = "tracing-subscriber")]
    fn parse_env_filter(s: &str) -> Result<EnvFilter, String> {
        s.parse()
            .map_err(|error| format!("Bad tracing filter: {error}"))
    }
}
