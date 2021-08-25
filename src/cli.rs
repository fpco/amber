use std::path::PathBuf;

use clap::Clap;
use once_cell::sync::Lazy;

pub fn init() -> Cmd {
    let cmd = Cmd::parse();
    cmd.opt.init_logger();
    cmd
}

#[derive(Clap, Debug)]
#[clap(version = VERSION_SHA.as_str())]
pub struct Cmd {
    #[clap(flatten)]
    pub opt: Opt,
    #[clap(subcommand)]
    pub sub: SubCommand,
}

#[derive(Clap, Debug)]
pub enum SubCommand {
    /// Initialize a new directory
    Init,
    /// Add or update a secret
    Encrypt {
        /// Key, must be all capital ASCII characters, digits, and underscores
        key: String,
        /// Value
        value: String,
    },
    /// Generate a new strong secret value, and add it to the repository
    Generate {
        /// Key, must be all capital ASCII characters, digits, and underscores
        key: String,
    },
    /// Remove a secret
    Remove {
        /// Key, must be all capital ASCII characters, digits, and underscores
        key: String,
    },
    /// Print all of the secrets
    Print {
        /// Secrets output style, possible values are: setenv, json, yaml, pure. The default is setenv.
        #[clap(long = "--style", default_value = "setenv")]
        style: PrintStyle,
    },
    /// Run a command with all of the secrets set as environment variables
    Exec {
        /// Command to run
        cmd: String,
        /// Command line arguments to pass to the command
        args: Vec<String>,
    },
}

#[derive(Clap, Debug)]
pub enum PrintStyle {
    /// Output with `export` prefix, can be evaled in shell.
    SetEnv,
    /// Output as object with `key` and `value` attributes.
    Json,
    /// Output as object with `key` and `value` attributes.
    Yaml,
}

impl core::str::FromStr for PrintStyle {
    type Err = clap::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "setenv" => Ok(PrintStyle::SetEnv),
            "json" => Ok(PrintStyle::Json),
            "yaml" => Ok(PrintStyle::Yaml),
            _ => Err(clap::Error::with_description(
                String::from("Invalid option for Print command"),
                clap::ErrorKind::InvalidValue,
            )),
        }
    }
}

static VERSION_SHA: Lazy<String> = Lazy::new(|| {
    format!(
        "{} (Git SHA1 {})",
        env!("CARGO_PKG_VERSION"),
        env!("VERGEN_GIT_SHA")
    )
});

/// Utility to store encrypted secrets in version trackable plain text files.
#[derive(Clap, Debug)]
pub struct Opt {
    /// Turn on verbose output
    #[clap(short, long, global = true)]
    pub verbose: bool,
    /// amber.yaml file location
    #[clap(long, default_value = "amber.yaml", global = true, env = "AMBER_YAML")]
    pub amber_yaml: PathBuf,
    /// Disable masking of secret values during exec
    #[clap(long, global = true)]
    pub unmasked: bool,
}

impl Opt {
    /// Initialize the logger based on command line settings
    pub fn init_logger(&self) {
        use env_logger::{Builder, Target};
        use log::LevelFilter::*;
        let mut builder = Builder::from_default_env();
        let level = if self.verbose { Debug } else { Info };
        builder.filter_module(env!("CARGO_CRATE_NAME"), level);
        builder.target(Target::Stderr).init();
    }
}
