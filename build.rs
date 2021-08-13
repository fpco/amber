use anyhow::*;
use vergen::{vergen, Config};

fn main() -> Result<()> {
    vergen(Config::default())
}
