# Deno Version Manager

Easy way to manage multiple active deno versions.

## Installation

You can install it using the installers below, or download a release binary from the [releases page](https://github.com/justjavac/dvm/releases).

**With Shell:**

```sh
curl -fsSL https://deno.land/x/dvm/install.sh | sh
```

**With PowerShell:**

```powershell
iwr https://deno.land/x/dvm/install.ps1 -useb | iex
```

## Usage

```
➜  ~  dvm --help
Deno Version Manager - Easy way to manage multiple active deno versions.

USAGE:
    dvm [SUBCOMMAND]

OPTIONS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    completions    Generate shell completions
    help           Prints this message or the help of the given subcommand(s)
    info           Show dvm info
    install        Install deno executable to given version [aliases: i]
    list           List installed versions, matching a given <version> if provided [aliases: ls]
    use            Use a given version

Example:
  dvm install 1.3.2     Install v1.3.2 release
  dvm install           Install the latest available version
  dvm use 1.0.0         Use v1.0.0 release
```

### Verify installation

To verify that dvm has been installed, do:

```bash
dvm -V
```

which should output dvm's version if the installation was successful.

### Initialisation

Calling `dvm` will creates an `~/.dvm/` directory if it doesn't exist,
and all installed versions of deno will put into `~/.dvm`.

```
➜  ~  dvm
Creating /Users/justjavac/.dvm
```

## Example

### Listing versions

List all installed versions:

```
➜  ~  dvm list
 * 0.1.0
   0.1.1
   0.1.2
```

The version with a asterisk(`*`) means that this version is the version currently in use.

### Switching version

```
➜  ~  dvm use 1.1.0
now use deno 1.1.0
➜  ~  dvm use 1.2.0
deno v1.2.0 is not installed. Use `dvm install 1.2.0` to install it first.
```
## Compatibility

- The Shell installer can be used on Windows with [Windows Subsystem for Linux](https://docs.microsoft.com/en-us/windows/wsl/about), [MSYS](https://www.msys2.org) or equivalent set of tools.

## Known Issues

### unzip is required

The program [`unzip`](https://linux.die.net/man/1/unzip) is a requirement for the Shell installer.

```sh
$ curl -fsSL https://deno.land/x/dvm/install.sh | sh
Error: unzip is required to install dvm (see: https://github.com/justjavac/dvm#unzip-is-required).
```

**When does this issue occur?**

During the `install.sh` process, `unzip` is used to extract the zip archive.

**How can this issue be fixed?**

You can install unzip via `brew install unzip` on MacOS or `apt-get install unzip -y` on Linux.

## License

Deno Version Manager(dvm) is released under the MIT License. See the bundled [LICENSE](./LICENSE) file for details.
