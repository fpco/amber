use anyhow::*;
use std::path::Path;
use vergen::EmitBuilder;

fn main() -> Result<()> {
    if Path::new(".git").exists() {
        EmitBuilder::builder().git_sha(true).emit()
    } else {
        Ok(())
    }
}
