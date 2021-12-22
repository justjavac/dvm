# Deno 版本管理工具

[English](https://github.com/qiuquanwu/dvm/) | 简体中文

简单管理多个活动 deno 版本的方式。

## 安装

您可以使用下面的安装程序安装它，或者从[发布页](https://github.com/justjavac/dvm/releases)面下载源文件.

**Shell 安装:**

```sh
curl -fsSL https://deno.land/x/dvm/install.sh | sh
```

**PowerShell 安装:**

```powershell
iwr https://deno.land/x/dvm/install.ps1 -useb | iex
```

**注意**: 如果你使用的是 Apple M1，请使用 `cargo install dvm` 手动编译，因为 Github Actions 目前还不支持 aarch64 架构。

## 使用

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
    list-remote    List released versions [aliases: ls-remote]
    uninstall      Uninstall a given version [aliases: rm]
    use            Use a given version

Example:
  dvm install 1.3.2     Install v1.3.2 release
  dvm install           Install the latest available version
  dvm use 1.0.0         Use v1.0.0 release
```

### 验证

要验证 dvm 是否已安装，输入:

```bash
dvm -V
```

如果输出 dvm 的版本，则已经安装成功.

### 初始化

使用 `dvm` 将创建 `~/.dvm/` 文件夹 ,并且所有已安装的 deno 版本都将放入中 `~/.dvm`文件夹之中.

```
➜  ~  dvm
Creating /Users/justjavac/.dvm
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

- Shell 安装程序可以在带有 [Windows Subsystem for Linux](https://docs.microsoft.com/en-us/windows/wsl/about), [MSYS](https://www.msys2.org) 或等效工具集的 Windows 上使用。

## 常见问题

### unzip is required

此项目需要依赖 [`unzip`](https://linux.die.net/man/1/unzip) 进行 Shell 安装.

```sh
$ curl -fsSL https://deno.land/x/dvm/install.sh | sh
Error: unzip is required to install dvm (see: https://github.com/justjavac/dvm#unzip-is-required).
```

**此问题何时出现？**

在运行 `install.sh` 过程中, 会使用 `unzip` 提取 zip 文件。

**如何解决？**

- MacOs 使用 `brew install unzip` 安装 unzip。
- Ubuntu，Debian 使用`apt-get install unzip -y` 安装 unzip。
- CentOS 使用 `yum install -y unzip zip` 安装 unzip。

## 开源协议

Deno 版本管理器 （dvm） 遵循 MIT 开源协议。有关详细信息，请参阅 [LICENSE](./LICENSE)。
