# Change log for Amber

## 0.1.4 (2022-11-29)

* Upgrade to clap v4
* Add `--only-secret-key` option for `amber init`
* Switch to crypto_box from sodiumoxide.

## 0.1.3 (2022-03-13)

* Add the `write-file` command

## 0.1.2 (2022-01-18)

* Allow `encrypt` subcommand to take secret value from `stdin` [#15](https://github.com/fpco/amber/issues/15)
* Amber searches the parent directory for the amber.yaml file if
  amber.yaml isn't present in the current working directory. This
  check is only done when no explicit amber-yaml is specificed (unless
  the specified amber yaml itself is amber.yaml which is the default
  value)

## 0.1.1 (2021-08-31)

* Add masking support [#1](https://github.com/fpco/amber/issues/1)
* Add subcommand `generate` [#7](https://github.com/fpco/amber/pull/7)
* Do `vergen` initialization only when `.git` directory is
  present. This makes amber easy to package for distributions like
  NixOS.
* Rework print subcommand to provide more output styles.
  [#11](https://github.com/fpco/amber/pull/11)

## 0.1.0 (2021-08)

Initial release
