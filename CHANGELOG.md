# Changelog

### 1.5.2 - [2021-12-22]

- use specify branch for installer script

### 1.5.1 - [2021-12-22]

- use .dvmrc (#60)

### 1.5.0 - [2021-12-22]

- add `list-remote`
- auto detect China mainland

### 1.4.7 - [2021-12-01]

- Detect aarch64 to support Apple M1 (#52)
- fix: version `1.10.0` should be greater than version `1.9.1` (#53)

## 1.4.6 - [2021-01-31]

- fix ANSI colors in Windows CMD and PowerShell

## 1.4.5 - [2021-01-29]

- feat: Add `dvm list-remote` alias `dvm ls-remote` (#47)

## 1.4.3 - [2021-01-27]

- reduce size

## 1.4.0 - [2021-01-26]

- feat: use `dl.deno.land` #43

## 1.3.0 - [2021-01-25]

- Feature: Add `dvm uninstall x.x.x`, alias `dvm rm x.x.x` #40

## 1.2.0 - [2021-01-03]

- feat: Support changing dvm root by DVM_DIR env #35
- fix: Filter non-SemVer versions in the list showed by `dvm ls` #36
- fix actions environment variable #37

## 1.1.10 - [2020-09-07]

- use jsdelivr (#28)

## 1.1.9 - [2020-09-01]

- Fix dir not exists #25

## 1.0.0 - [2020-08-31]

- rewrite use rust

## 0.3.1 - [2019-06-06]

- Fix: double slash in download url [#5](https://github.com/justjavac/dvm/pull/5)
- Resole security alert [#8](https://github.com/justjavac/dvm/pull/8)

## 0.1.6 - [2018-09-05]

- Add `DVM_PATH` env var
- Replace compressing with extract-zip
- Ignore package-lock.json
- Modify "downloading" position
- Fix: dest path on osx
- Fix: spelling mistake

## 0.1.4 - [2018-09-04]

- Initial release
