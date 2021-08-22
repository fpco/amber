use anyhow::*;
use std::path::Path;
use vergen::{vergen, Config};

fn main() -> Result<()> {
    if Path::new(".git").exists() {
        vergen(Config::default())
    } else {
        Ok(())
    }
}
