//! Logic for handling the masking of secret values when running an executable.

use std::{
    io::{Read, Write},
    process::{Command, Stdio},
    sync::Arc,
    thread::spawn,
};

use aho_corasick::AhoCorasick;
use anyhow::*;

/// Run the given command with stdout and stderr values masked.
pub fn run_masked(mut cmd: Command, secrets: &[impl AsRef<[u8]>]) -> Result<()> {
    // Unfortunately LeftmostLongest isn't supported by the streaming interface
    // let ac = AhoCorasickBuilder::new().auto_configure(secrets).match_kind(MatchKind::LeftmostLongest).build(secrets);

    let ac = AhoCorasick::new(secrets.iter());
    let mut child = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Unable to spawn child process")?;

    let stdout = child.stdout.take().context("No stdout available")?;
    let stderr = child.stderr.take().context("No stderr available")?;

    let mut replacements = Vec::new();
    for secret in secrets {
        let len = secret.as_ref().len();
        let replacement: Vec<u8> = std::iter::repeat(b'*').take(len).collect();
        replacements.push(replacement);
    }
    let replacements = Arc::new(replacements);

    let ac_clone = ac.clone();
    let replacements_clone = replacements.clone();
    let handle_out =
        spawn(move || mask_stream(stdout, std::io::stdout(), ac_clone, &*replacements_clone));
    let handle_err = spawn(move || mask_stream(stderr, std::io::stderr(), ac, &*replacements));

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

fn mask_stream<R>(
    input: impl Read,
    output: impl Write,
    ac: AhoCorasick,
    replacements: impl AsRef<[R]>,
) -> Result<()>
where
    R: AsRef<[u8]>,
{
    ac.stream_replace_all(input, output, replacements.as_ref())
        .context("Error while masking a stream")
}
