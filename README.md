Deno Version Manager
=====

Switch between different versions of [Deno](https://github.com/denoland/deno).

Installation
------------

Currently you can use npm to install dvm:

```sh
npm install -g dvm
```

Usage
-----

```
➜  ~  dvm --help

Usage: dvm [options] [command]

Options:

  -v, --version      output the version number
  -h, --help         output usage information

Commands:

  arch               Show if deno is running in 32 or 64 bit mode.
  list               List all installed versions
  install <version>  Install deno <version>
  use [version]      Switch to use the specified version.
```

### Verify installation

To verify that dvm has been installed, do:

```bash
dvm -v
```

which should output dvm's version if the installation was successful.

### Initialisation

Calling `dvm` will creates an `~/.dvm/` directory if it doesn't exist,
and all installed versions of deno will put into `~/.dvm`.

```
➜  ~  dvm
Creating /Users/justjavac/.dvm
```

Note For Windows Users
----------------------

You may have to run `dvm` in a shell (cmd, PowerShell, Git Bash, etc) with
elevated (Administrative) privileges to get it to run.

```
➜  ~  dvm use 0.1.2
You may have to run dvm in a shell (cmd, PowerShell, Git Bash, etc) with elevated (Administrative) privileges to get it to run.
```

Known deno download registry Mirrors
---------------------

*TODO*

For your convenience, when you use `dvm install` to install a specific version of deno, you can pick a registry. Currently we provide these registries built in:

* [deno](https://github.com/denoland/deno): `dvm install 0.1.2 -r deno`
* [denocn](https://deno.js.cn): `dvm install 0.1.2 -r denocn`

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
➜  ~  dvm use 0.1.2
now use 0.1.2
➜  ~  dvm use 0.0.2
deno v0.0.2 is not installed. Use `dvm install 0.0.2` to install it first.
```

## Credits

* GitHub ([justjavac](http://github.com/justjavac))
* Twitter ([@justjavac](http://twitter.com/justjavac))
* Weibo ([@justjavac](http://weibo.com/justjavac))

## License

Deno Version Manager(dvm) is released under the GPL License. See the bundled [LICENSE](./LICENSE) file for details.
