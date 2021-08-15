#[cfg(feature = "aws")]
mod aws;
mod cli;
mod config;
mod exec;

use anyhow::*;
use exec::CommandExecExt;

fn main() -> Result<()> {
    let cmd = cli::init();
    log::debug!("{:?}", cmd);
    match cmd.sub {
        cli::SubCommand::Init => init(cmd.opt),
        cli::SubCommand::Encrypt { key, value } => encrypt(cmd.opt, key, value),
        cli::SubCommand::Remove { key } => remove(cmd.opt, key),
        cli::SubCommand::Print => print(cmd.opt),
        cli::SubCommand::Exec { cmd: cmd_, args } => exec(cmd.opt, cmd_, args),
    }
}

fn init(opt: cli::Opt) -> Result<()> {
    let (secret_key_, config) = config::Config::new();
    let secret_key = sodiumoxide::hex::encode(&secret_key_);

    config.save(&opt.amber_yaml)?;

    eprintln!("Your secret key is: {}", secret_key);
    eprintln!(
        "Please save this key immediately! If you lose it, you will lose access to your secrets."
    );
    eprintln!("Recommendation: keep it in a password manager");
    eprintln!("If you're using this for CI, please update your CI configuration with a secret environment variable");
    println!("export {}={}", config::SECRET_KEY_ENV, secret_key);

    match opt.secret_key_source {
        config::SecretKeySource::Env => (),
        config::SecretKeySource::Aws => {
            #[cfg(feature = "aws")]
            aws::save(&opt.aws_region, config.get_public_key(), &secret_key_)?;
        }
        config::SecretKeySource::Azure => todo!(),
    }

    Ok(())
}

fn validate_key(key: &str) -> Result<()> {
    ensure!(!key.is_empty(), "Cannot provide an empty key");
    if key
        .chars()
        .all(|c| c.is_ascii_digit() || c.is_ascii_uppercase() || c == '_')
    {
        Ok(())
    } else {
        Err(anyhow!(
            "Key must be exclusively upper case ASCII, digits, and underscores"
        ))
    }
}

fn encrypt(opt: cli::Opt, key: String, value: String) -> Result<()> {
    validate_key(&key)?;
    let mut config = config::Config::load(&opt.amber_yaml)?;
    config.encrypt(key, &value);
    config.save(&opt.amber_yaml)
}

fn remove(opt: cli::Opt, key: String) -> Result<()> {
    validate_key(&key)?;
    let mut config = config::Config::load(&opt.amber_yaml)?;
    config.remove(&key);
    config.save(&opt.amber_yaml)
}

fn print(opt: cli::Opt) -> Result<()> {
    let config = config::Config::load(&opt.amber_yaml)?;
    let secret = config.load_secret_key(&opt)?;
    let pairs: Result<Vec<_>> = config.iter_secrets(&secret).collect();
    let mut pairs = pairs?;
    pairs.sort_by(|x, y| x.0.cmp(y.0));
    pairs
        .iter()
        .for_each(|(key, value)| println!("export {}={:?}", key, value));

    Ok(())
}

fn exec(opt: cli::Opt, cmd: String, args: Vec<String>) -> Result<()> {
    let config = config::Config::load(&opt.amber_yaml)?;
    let secret_key = config.load_secret_key(&opt)?;

    let mut cmd = std::process::Command::new(cmd);
    cmd.args(args);

    for pair in config.iter_secrets(&secret_key) {
        let (name, value) = pair?;
        log::debug!("Setting env var in child process: {}", name);
        cmd.env(name, value);
    }

    cmd.emulate_exec("Launching child process")?;

    Ok(())
}
