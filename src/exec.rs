use anyhow::*;

pub trait CommandExecExt {
    fn emulate_exec(&mut self, desc: &str) -> Result<()>;
}

impl CommandExecExt for std::process::Command {
    #[cfg(not(unix))]
    fn emulate_exec(&mut self, desc: &str) -> Result<()> {
        let mut child = self.spawn().with_context(|| desc.to_owned())?;

        let status = child.wait().with_context(|| desc.to_owned())?;

        let code = status
            .code()
            .ok_or_else(|| anyhow!("Unexpected exit status from {}", desc))?;

        std::process::exit(code)
    }

    #[cfg(unix)]
    fn emulate_exec(&mut self, desc: &str) -> Result<()> {
        use std::os::unix::process::CommandExt;
        let err = self.exec();
        Err(err).context(desc.to_owned())
    }
}
