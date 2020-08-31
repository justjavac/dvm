#!/bin/sh
# Copyright 2019 the Deno authors. All rights reserved. MIT license.
# Copyright 2020 justjavac. All rights reserved. MIT license.
# TODO(everyone): Keep this script simple and easily auditable.

set -e

if [ "$(uname -m)" != "x86_64" ]; then
	echo "Error: Unsupported architecture $(uname -m). Only x64 binaries are available." 1>&2
	exit 1
fi

if ! command -v unzip >/dev/null; then
	echo "Error: unzip is required to install Dvm (see: https://github.com/justjavac/dvm#unzip-is-required)." 1>&2
	exit 1
fi

if [ "$OS" = "Windows_NT" ]; then
	target="x86_64-pc-windows-msvc"
else
	case $(uname -s) in
	Darwin) target="x86_64-apple-darwin" ;;
	*) target="x86_64-unknown-linux-gnu" ;;
	esac
fi

if [ $# -eq 0 ]; then
	dvm_asset_path=$(
		curl -sSf https://github.com/justjavac/dvm/releases |
			grep -o "/justjavac/dvm/releases/download/.*/dvm-${target}\\.zip" |
			head -n 1
	)
	if [ ! "$dvm_asset_path" ]; then
		echo "Error: Unable to find latest dvm release on GitHub." 1>&2
		exit 1
	fi
	dvm_uri="https://github.com${dvm_asset_path}"
else
	dvm_uri="https://github.com/justjavac/dvm/releases/download/${1}/dvm-${target}.zip"
fi

deno_install="${DENO_INSTALL:-$HOME/.deno}"
dvm_dir="${DVM_DIR:-$HOME/.dvm}"
dvm_bin_dir="$dvm_dir/bin"
exe="$dvm_bin_dir/dvm"

if [ ! -d "$dvm_bin_dir" ]; then
	mkdir -p "$dvm_bin_dir"
fi

# curl --fail --location --progress-bar --output "$exe.zip" "$dvm_uri"
# cd "$dvm_bin_dir"
# unzip -o "$exe.zip"
# chmod +x "$exe"
# rm "$exe.zip"

case $SHELL in
/bin/zsh) shell_profile=".zshrc" ;;
*) shell_profile=".bash_profile" ;;
esac

if [ ! $DENO_INSTALL ];then
    command echo "export DENO_INSTALL=\"$deno_install\"" >> "$HOME/$shell_profile"
    command echo "export PATH=\"\$DENO_INSTALL/bin:\$PATH\"" >> "$HOME/$shell_profile"
fi

if [ ! $DVM_DIR ];then
    command echo "export DVM_DIR=\"$dvm_dir\"" >> "$HOME/$shell_profile"
    command echo "export PATH=\"\$DVM_DIR/bin:\$PATH\"" >> "$HOME/$shell_profile"
fi

echo "Dvm was installed successfully to $exe"
if command -v dvm >/dev/null; then
	echo "Run 'dvm --help' to get started"
else
	echo "Reopen your shell, or run 'source $HOME/$shell_profile' to get started"
fi
