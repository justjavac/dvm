# Deno 版本管理工具

[English](https://github.com/qiuquanwu/dvm/) | 简体中文

简单管理多个活动 deno 版本的方式。

## 安装

您可以使用下面的安装程序安装它，或者从[发布页](https://github.com/justjavac/dvm/releases)面下载源文件。

**Shell 安装：**

```sh
curl -fsSL https://deno.land/x/dvm/install.sh | sh
```

**PowerShell 安装：**

```powershell
iwr https://deno.land/x/dvm/install.ps1 -useb | iex
```

## 使用

```plain
➜  ~  dvm --help
Deno Version Manager - Easy way to manage multiple active deno versions.

USAGE:
    dvm <SUBCOMMAND>

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information

SUBCOMMANDS:
    completions    Generate shell completions
    help           Print this message or the help of the given subcommand(s)
    info           Show dvm info.
    install        Install deno executable to the given version. [aliases: i, add]
    list           List installed versions, matching a given <version> if provided [aliases: ls,
                   ll, la]
    list-remote    List released versions [aliases: lr, ls-remote]
    uninstall      Uninstall a given version [aliases: un, unlink, rm, remove]
    use            Use a given version

EXAMPLE:
  dvm install 1.3.2     Install v1.3.2 release
  dvm install           Install the latest available version
  dvm use 1.0.0         Use v1.0.0 release

NOTE:
  To remove, delete, or uninstall dvm - just remove the `$DVM_DIR` folder (usually `~/.dvm`)
```

### 验证

要验证 dvm 是否已安装，输入：

```bash
dvm -V
```

如果输出 dvm 的版本，则已经安装成功。

### 初始化

使用 `dvm` 将创建 `~/.dvm/` 文件夹，并且所有已安装的 deno 版本都将放入中 `~/.dvm`文件夹之中。

```
➜  ~  dvm
Creating /Users/justjavac/.dvm
```

### .dvmrc

你可以在项目根目录创建一个 `.dvmrc` 文件，然后把 Deno 的版本号写在里面。这样当你运行 `dvm use`、`dvm install`
命令时，如果没有在命令行上提供版本参数，那么 dvm 会自动从 `.dvmrc` 文件读取 Deno 的版本号。

举个例子，我们想把当前仓库的 Deno 版本号设置为 `1.17.0`，那么我们可以在根目录运行：

```bash
echo "1.17.0" > .dvmrc
```

然后运行 dvm：

```plain
$ dvm use
Found '.dvmrc' with version <1.17.0>
Now using deno 1.17.0
```

## 举个例子

### 查看版本

列出所有安装的版本：

```
➜  ~  dvm list
 * 0.1.0
   0.1.1
   0.1.2
```

带（*）的版本表示此版本是当前使用的版本。

### 切换版本

```
➜  ~  dvm use 1.1.0
now use deno 1.1.0
➜  ~  dvm use 1.2.0
deno v1.2.0 is not installed. Use `dvm install 1.2.0` to install it first.
```

## 兼容性

- Shell 安装程序可以在带有
  [Windows Subsystem for Linux](https://docs.microsoft.com/en-us/windows/wsl/about)，
  [MSYS](https://www.msys2.org) 或等效工具集的 Windows 上使用。

## 常见问题

### unzip is required

此项目需要依赖 [`unzip`](https://linux.die.net/man/1/unzip) 进行 Shell 安装。

```sh
$ curl -fsSL https://deno.land/x/dvm/install.sh | sh
Error: unzip is required to install dvm (see: https://github.com/justjavac/dvm#unzip-is-required).
```

**此问题何时出现？**

在运行 `install.sh` 过程中，会使用 `unzip` 提取 zip 文件。

**如何解决？**

- MacOs 使用 `brew install unzip` 安装 unzip。
- Ubuntu，Debian 使用`apt-get install unzip -y` 安装 unzip。
- CentOS 使用 `yum install -y unzip zip` 安装 unzip。

## 开源协议

Deno 版本管理器 （dvm） 遵循 MIT 开源协议。有关详细信息，请参阅 [LICENSE](./LICENSE)。
