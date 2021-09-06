mod cli;
mod config;
mod exec;
mod mask;

use std::io::Read;

use anyhow::*;
use exec::CommandExecExt;
use serde::Serialize;

#[derive(Serialize)]
struct KeyValue<'a> {
    key: &'a str,
    value: &'a str,
}

impl<'a, K, V> From<&'a (K, V)> for KeyValue<'a>
where
    K: AsRef<str>,
    V: AsRef<str>,
{
    fn from((key, value): &'a (K, V)) -> Self {
        KeyValue {
            key: key.as_ref(),
            value: value.as_ref(),
        }
    }
}

fn main() -> Result<()> {
    let cmd = cli::init();
    log::debug!("{:?}", cmd);
    match cmd.sub {
        cli::SubCommand::Init => init(cmd.opt),
        cli::SubCommand::Encrypt { key, value } => encrypt(cmd.opt, key, value),
        cli::SubCommand::Generate { key } => generate(cmd.opt, key),
        cli::SubCommand::Remove { key } => remove(cmd.opt, key),
        cli::SubCommand::Print { style } => print(cmd.opt, style),
        cli::SubCommand::Exec { cmd: cmd_, args } => exec(cmd.opt, cmd_, args),
    }
}

fn init(opt: cli::Opt) -> Result<()> {
    let (secret_key, config) = config::Config::new();
    let secret_key = sodiumoxide::hex::encode(secret_key);

    config.save(&opt.amber_yaml)?;

    eprintln!("Your secret key is: {}", secret_key);
    eprintln!(
        "Please save this key immediately! If you lose it, you will lose access to your secrets."
    );
    eprintln!("Recommendation: keep it in a password manager");
    eprintln!("If you're using this for CI, please update your CI configuration with a secret environment variable");
    println!("export {}={}", config::SECRET_KEY_ENV, secret_key);

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

fn encrypt(opt: cli::Opt, key: String, value: Option<String>) -> Result<()> {
    validate_key(&key)?;
    let mut config = config::Config::load(&opt.amber_yaml)?;
    let value = value.map_or_else(
        || {
            log::debug!("No value provided on command line, taking from stdin");
            eprintln!("Enter secret value (send EOF when done)");
            eprintln!();
            let stdin = std::io::stdin();
            let mut stdin = stdin.lock();
            let mut buffer = String::new();
            stdin.read_to_string(&mut buffer).map(|_size| buffer)
        },
        Ok,
    )?;
    config.encrypt(key, &value);
    config.save(&opt.amber_yaml)
}

fn generate(opt: cli::Opt, key: String) -> Result<()> {
    let value = sodiumoxide::randombytes::randombytes(16);
    let value = sodiumoxide::base64::encode(value, sodiumoxide::base64::Variant::UrlSafe);
    let msg = format!("Your new secret value is {}: {}", key, value);
    let res = encrypt(opt, key, Some(value))?;
    println!("{}", &msg);
    Ok(res)
}

fn remove(opt: cli::Opt, key: String) -> Result<()> {
    validate_key(&key)?;
    let mut config = config::Config::load(&opt.amber_yaml)?;
    config.remove(&key);
    config.save(&opt.amber_yaml)
}

fn print(opt: cli::Opt, style: cli::PrintStyle) -> Result<()> {
    let config = config::Config::load(&opt.amber_yaml)?;
    let secret = config.load_secret_key()?;
    let pairs: Result<Vec<_>> = config.iter_secrets(&secret).collect();
    let mut pairs = pairs?;
    pairs.sort_by(|x, y| x.0.cmp(y.0));

    fn to_objs<'a, K, V, I>(p: I) -> Vec<KeyValue<'a>>
    where
        I: IntoIterator<Item = &'a (K, V)>,
        K: AsRef<str> + 'a,
        V: AsRef<str> + 'a,
    {
        p.into_iter().map(KeyValue::from).collect::<Vec<_>>()
    }
    match style {
        cli::PrintStyle::SetEnv => pairs
            .iter()
            .for_each(|(key, value)| println!("export {}={:?}", key, value)),
        cli::PrintStyle::Json => {
            let secrets = to_objs(&pairs);
            serde_json::to_writer(std::io::stdout(), &secrets)?;
        }
        cli::PrintStyle::Yaml => {
            let secrets = to_objs(&pairs);
            serde_yaml::to_writer(std::io::stdout(), &secrets)?;
        }
    }

    Ok(())
}

fn exec(opt: cli::Opt, cmd: String, args: Vec<String>) -> Result<()> {
    let config = config::Config::load(&opt.amber_yaml)?;
    let secret_key = config.load_secret_key()?;

    let mut cmd = std::process::Command::new(cmd);
    cmd.args(args);

    let mut secrets = Vec::new();
    for pair in config.iter_secrets(&secret_key) {
        let (name, value) = pair?;
        log::debug!("Setting env var in child process: {}", name);
        cmd.env(name, &value);
        if !opt.unmasked {
            secrets.push(value);
        }
    }

    if opt.unmasked {
        cmd.emulate_exec("Launching child process")?;
    } else {
        mask::run_masked(cmd, &secrets)?;
    }

    Ok(())
}
