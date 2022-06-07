#!/bin/sh
# Copyright 2019 the Deno authors. All rights reserved. MIT license.
# Copyright 2020 justjavac. All rights reserved. MIT license.
# TODO(everyone): Keep this script simple and easily auditable.

set -e

if ! command -v unzip >/dev/null; then
	echo "Error: unzip is required to install Dvm (see: https://github.com/justjavac/dvm#unzip-is-required)." 1>&2
	exit 1
fi

if [ "$OS" = "Windows_NT" ]; then
	target="x86_64-pc-windows-msvc"
else
	case $(uname -sm) in
	"Darwin x86_64") target="x86_64-apple-darwin" ;;
	"Darwin arm64") target="aarch64-apple-darwin" ;;
	"Linux x86_64") target="x86_64-unknown-linux-gnu" ;;
	*) echo "Unsupported OS + CPU combination: $(uname -sm)"; exit 1 ;;
	esac
fi

dvm_uri="https://cdn.jsdelivr.net/gh/justjavac/dvm_releases@main/dvm-${target}.zip"

deno_install="${DENO_INSTALL:-$HOME/.deno}"
dvm_dir="${DVM_DIR:-$HOME/.dvm}"
dvm_bin_dir="$dvm_dir/bin"
exe="$dvm_bin_dir/dvm"

if [ ! -d "$dvm_bin_dir" ]; then
	mkdir -p "$dvm_bin_dir"
fi

if [ "$1" = "" ]; then
	cd "$dvm_bin_dir"
	curl --fail --location --progress-bar -k --output "$exe.zip" "$dvm_uri"
	unzip -o "$exe.zip"
	rm "$exe.zip"
else
	echo "Install path override detected: $1"
	if [ ! -f "$1" ]; then
		echo "File does not exist: $1"
		exit 1
	fi
	cp "$1" "$exe"
fi
cd "$dvm_bin_dir"
chmod +x "$exe"

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
	command dvm doctor
	echo "Run 'dvm --help' to get started."
else
	echo "Reopen your shell, or run 'source $HOME/$shell_profile' to get started"
fi
