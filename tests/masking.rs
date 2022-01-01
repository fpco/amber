use assert_cmd::prelude::*;
use std::process::Command;

const AMBER_YAML: &str = "assets/amber-masking.yaml";
const SECRET_KEY: &str = "ac2af4852f3de2dc6feb19b718d1cbf6c64c1ef618dafaf2b0a89cadcde240ac";
const TO_MASK: &str = include_str!("../assets/tomask.txt");
const MASKED: &str = include_str!("../assets/masked.txt");

#[test]
fn masking() {
    let output = Command::cargo_bin("amber")
        .unwrap()
        .arg("exec")
        .arg("cat")
        .arg("assets/tomask.txt")
        .env("AMBER_YAML", AMBER_YAML)
        .env("AMBER_SECRET", SECRET_KEY)
        .output()
        .unwrap();
    if !output.status.success() {
        eprintln!("{}", std::str::from_utf8(&output.stderr).unwrap());
        panic!("Did not exit successfully");
    }
    assert_eq!(std::str::from_utf8(&output.stdout).unwrap(), MASKED);
}

#[test]
fn disable_masking() {
    let output = Command::cargo_bin("amber")
        .unwrap()
        .arg("exec")
        .arg("--unmasked")
        .arg("cat")
        .arg("assets/tomask.txt")
        .env("AMBER_YAML", AMBER_YAML)
        .env("AMBER_SECRET", SECRET_KEY)
        .output()
        .unwrap();
    if !output.status.success() {
        eprintln!("{}", std::str::from_utf8(&output.stderr).unwrap());
        panic!("Did not exit successfully");
    }
    assert_eq!(std::str::from_utf8(&output.stdout).unwrap(), TO_MASK);
}
