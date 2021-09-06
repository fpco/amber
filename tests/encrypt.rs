use std::io::Write;
use std::{path::Path, process::Stdio};

const AMBER_YAML: &str = "assets/amber-encrypt.yaml";
const SECRET_KEY: &str = "2a0fb64171010cd4584e2b658fc0a5effca4cd9ada2b2eea0262356852c60872";

fn temp_amber_yaml() -> tempfile::TempPath {
    let path = tempfile::NamedTempFile::new().unwrap().into_temp_path();
    std::fs::copy(AMBER_YAML, &path).unwrap();
    path
}

#[derive(serde::Deserialize, PartialEq, Eq, Debug)]
struct Pair {
    key: String,
    value: String,
}

fn get_vars(path: impl AsRef<Path>) -> Vec<Pair> {
    let output = std::process::Command::new("cargo")
        .arg("run")
        .arg("--")
        .arg("print")
        .arg("--style")
        .arg("json")
        .env("AMBER_YAML", path.as_ref())
        .env("AMBER_SECRET", SECRET_KEY)
        .output()
        .unwrap();
    if !output.status.success() {
        eprintln!("{}", std::str::from_utf8(&output.stderr).unwrap());
        panic!("Did not print successfully");
    }
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn empty_file() {
    let temp = temp_amber_yaml();
    assert_eq!(get_vars(&temp), vec![]);
}

#[test]
fn encrypt_cli() {
    let temp = temp_amber_yaml();
    let status = std::process::Command::new("cargo")
        .arg("run")
        .arg("--")
        .arg("encrypt")
        .arg("FOO")
        .arg("foovalue")
        .env("AMBER_YAML", temp.as_os_str())
        .status()
        .unwrap();
    assert!(status.success());
    assert_eq!(
        get_vars(&temp),
        vec![Pair {
            key: "FOO".to_owned(),
            value: "foovalue".to_owned(),
        }]
    );
}

#[test]
fn encrypt_stdin() {
    let temp = temp_amber_yaml();
    let mut child = std::process::Command::new("cargo")
        .arg("run")
        .arg("--")
        .arg("encrypt")
        .arg("FOO")
        .env("AMBER_YAML", temp.as_os_str())
        .stdin(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    write!(&mut stdin, "foovalue via stdin").unwrap();
    std::mem::drop(stdin);
    let status = child.wait().unwrap();
    assert!(status.success());
    assert_eq!(
        get_vars(&temp),
        vec![Pair {
            key: "FOO".to_owned(),
            value: "foovalue via stdin".to_owned(),
        }]
    );
}
