//! Logic for handling the masking of secret values when running an executable.

use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    thread::spawn,
};

use aho_corasick::{AhoCorasick, Match};
use anyhow::*;

/// Run the given command with stdout and stderr values masked.
pub fn run_masked(mut cmd: Command, secrets: &[impl AsRef<[u8]>]) -> Result<()> {
    // Unfortunately LeftmostLongest isn't supported by the streaming interface
    // let ac = AhoCorasickBuilder::new().auto_configure(secrets).match_kind(MatchKind::LeftmostLongest).build(secrets);

    let ac = AhoCorasick::new(secrets.iter()).context("Error creating AhoCorasick type")?;
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Unable to spawn child process")?;

    let stdout = child.stdout.take().context("No stdout available")?;
    let stderr = child.stderr.take().context("No stderr available")?;

    let ac_clone = ac.clone();
    let handle_out = spawn(move || mask_stream(stdout, std::io::stdout(), ac_clone));
    let handle_err = spawn(move || mask_stream(stderr, std::io::stderr(), ac));

    handle_out
        .join()
        .map_err(|e| anyhow!("stdout thread panicked: {:?}", e))??;
    handle_err
        .join()
        .map_err(|e| anyhow!("stderr thread panicked: {:?}", e))??;

    let exit = child.wait().context("Unable to wait for child to exit")?;

    if exit.success() {
        Ok(())
    } else {
        match exit.code() {
            Some(ec) => std::process::exit(ec),
            None => Err(anyhow!("Child process died with unknown error code")),
        }
    }
}

fn mask_stream(input: impl Read, output: impl Write, ac: AhoCorasick) -> Result<()> {
    ac.try_stream_replace_all_with(input, output, replacer)
        .context("Error while masking a stream")
}

/// Always use the same length of masked value to avoid exposing information
/// about the secret length.
fn replacer(_: &Match, _: &[u8], w: &mut impl Write) -> std::io::Result<()> {
    w.write_all(b"******")
}
