# amber

[![Rust](https://github.com/fpco/amber/actions/workflows/rust.yml/badge.svg)](https://github.com/fpco/amber/actions/workflows/rust.yml)

Manage secret values in-repo via public key cryptography. See [the announcement blog post](https://www.fpcomplete.com/blog/announcing-amber-ci-secret-tool/) for more motivation.

Amber provides the ability to securely store secret data in a plain-text file. Secrets can be encrypted by anyone with access to the file, without the ability to read those files without a secret key. The file format is a plain text YAML file which minimizes diffs on value changes, making it amenable to tracking changes in version control.

The primary use case for Amber is storing secret values for Continuous Integration systems. In most CI secrets management systems, there is no way to track the changes in values over time. With Amber, the public key and encrypted values live inside the repo, ensuring future runs of the same commit will either fail (if you've misplaced/changed the key) or have identical inputs.

## Install

You can install from source by [installing Rust](https://www.rust-lang.org/tools/install) and running `cargo install --git https://github.com/fpco/amber`. Binaries are available on the [release page](https://github.com/fpco/amber/releases). Place the executable on your `PATH` and ensure that the executable bit is set (for non-Windows platforms).

## Usage

Running `amber --help` will give you full, up to date set of instructions. The `--amber-yaml` option, or the `AMBER_YAML` environment variable, can be used to specify the location of the file containing your secret values. If unspecified, it will default to `amber.yaml`. The typical workflow is:

* `amber init` to create a new secret key and `amber.yaml` file.
* Securely store that secret key, such as in a password manager. Additionally, if desired, put that secret key in your CI system's secrets.
* Add additional secrets with `amber encrypt`.
* Commit your `amber.yaml` file into your repository.
* Within your CI scripts, or when using your secrets on your own system:
    * Set the `AMBER_SECRET` environment variable to your secret key.
    * Use `amber print` to see a list of your secrets.
    * Use `amber exec ...` to execute subcommands with the secrets available.
* Over time, use `amber encrypt` to add new secrets or update existing secrets, and `amber remove` to remove a secret entirely.
* By storing the secrets in Git, you'll always be able to recover old secret values.

Here's a sample shell session:

```shellsession
$ amber init
Your secret key is: 15aa07775395303732870cff2cc35c26f94af3344cf0f85d230aa004234d9764
Please save this key immediately! If you lose it, you will lose access to your secrets.
Recommendation: keep it in a password manager
If you're using this for CI, please update your CI configuration with a secret environment variable
export AMBER_SECRET=15aa07775395303732870cff2cc35c26f94af3344cf0f85d230aa004234d9764
$ amber encrypt PASSWORD deadbeef
$ amber print
Error: Error loading secret key from environment variable AMBER_SECRET

Caused by:
    environment variable not found
$ export AMBER_SECRET=15aa07775395303732870cff2cc35c26f94af3344cf0f85d230aa004234d9764
$ amber print
export PASSWORD="deadbeef"
$ amber exec -- sh -c 'echo $PASSWORD'
deadbeef
$ cat amber.yaml
---
file_format_version: 1
public_key: 9a4eb57571201fe413a5a9d583a070d180669928f0b98152ad93454cf5079860
secrets:
  - name: PASSWORD
    sha256: 2baf1f40105d9501fe319a8ec463fdf4325a2a5df445adf3f572f626253678c9
    cipher: c7f3d90e15b2d37801055d9773e6bd1e4b36120987bf31c6f111d5d69acb6d020a5f532ea035c272465f2a6e43c55fb009bf03a5c7a93581
$ amber encrypt PASSWORD deadbeef
[2021-08-13T10:45:13Z INFO  amber::config] New value matches old value, doing nothing
$ amber encrypt PASSWORD deadbeef2
[2021-08-13T10:45:16Z WARN  amber::config] Overwriting old secret value
$ amber print
export PASSWORD="deadbeef2"
$ amber remove PASSWORD
$ amber print
$ cat amber.yaml
---
file_format_version: 1
public_key: 9a4eb57571201fe413a5a9d583a070d180669928f0b98152ad93454cf5079860
secrets: []
```

## Authors

This tool was written by the [FP Complete](https://www.fpcomplete.com/) engineering team. It was originally part of a deployment system for our [Kube360 Kubernetes software collection](https://www.fpcomplete.com/products/kube360/). We decided to extract the generalizable parts to a standalone tool to improve Continuous Integration workflows.

If you have a use case outside of CI, or additional features you think would fit in well, please let us know in the issue tracker!
