#!/usr/bin/env node

/* eslint-disable no-console */

const ver = process.versions.node;
const majorVer = parseInt(ver.split(".")[0], 10);

if (majorVer < 8) {
  console.error(
    "Node version %d is not supported, please use Node.js 8.0 or higher.",
    ver
  );
  process.exit(1);
}

const is_admin = require("is-admin");

const path = require("path");
const fs = require("fs");
const os = require("os");

const program = require("commander");
const npm_prefix = require("npm-prefix")();
const ProgressBar = require("progress");
const compressing = require("compressing");
const request = require("request");

const pkg = require("../package.json");
const registries = require("../registries.json");
// let dvmrc = path.join(os.homedir(), ".dvmrc");

const DVM_PATH = process.env.DVM_PATH || path.join(os.homedir(), ".dvm");
const DENO_EXECUTOR = "deno" + (process.platform === "win32" ? ".exe" : "");

const DENO_PATH = path.join(
  npm_prefix,
  process.platform === "win32" ? "" : "bin",
  DENO_EXECUTOR
);

if (!fs.existsSync(DVM_PATH)) {
  console.log("Creating %s", DVM_PATH);
  fs.mkdirSync(DVM_PATH, "0777");
}

program.version(pkg.version, "-v, --version");

program
  .command("arch")
  //   .option("-s, --setup_mode [mode]", "Which setup mode to use")
  .description("Show if deno is running in 32 or 64 bit mode.")
  .action(() => {
    console.log("System Arch: %s_%s.", process.platform, process.arch);
  });

program
  .command("list")
  .description("List all installed versions")
  .action(function() {
    list_verions();
  });

program
  .command("install <version>")
  .description("Install deno <version>")
  .action(function(version) {
    if (fs.existsSync(path.join(DVM_PATH, version))) {
      console.log(
        "Deno v%s is already installed, run `dvm use %s` to use this version.",
        version,
        version
      );
      process.exit(1);
    }

    download(version)
      .then(filePath => extractDownload(filePath, path.join(DVM_PATH, version)))
      .then(() => {
        link(version);
      })
      .catch(msg => {
        console.error(msg, version);
      });
  })
  .on("--help", function() {
    console.log("  Examples:");
    console.log();
    console.log("    $ dvm install 0.1.0");
    console.log("    $ dvm install latest");
    console.log();
  });

program
  .command("use [version]")
  .description("Switch to use the specified version.")
  .action(version => {
    if (version !== undefined) {
      link(version);
      return;
    }

    const current = current_verion();

    if (current === "") {
      console.log(
        "There does not seem to install any version of deno.\nplease use `dvm install [version]` to install."
      );
      return;
    }

    console.log("current version is %s.\n", current);
    console.log("Switch to another version using `dvm use [version]`");
    console.log("List all installed versions using `dvm list`");
  })
  .on("--help, -h", function() {
    console.log("  Examples:");
    console.log();
    console.log("    $ dvm use 0.1.0");
    console.log("    $ dvm use latest");
    console.log();
  });

program.on("option:verbose", function() {
  process.env.VERBOSE = this.verbose;
});

program.on("command:*", function() {
  console.error(
    "Invalid command: %s\nSee --help or -h for a list of available commands.",
    program.args.join(" ")
  );
  process.exit(1);
});

program.parse(process.argv);

if (process.argv.length === 2) {
  program.help();
}

function list_verions() {
  const current = current_verion();
  fs.readdirSync(DVM_PATH).forEach(function(denos) {
    if (denos[0] !== ".") {
      console.log(" %s %s", current == denos ? "*" : " ", denos);
    }
  });
}

function current_verion() {
  let version = "";

  try {
    const link = fs.readlinkSync(DENO_PATH);
    version = link && path.basename(path.resolve(link, ".."));
  } catch (e) {} /* eslint-disable-line no-empty */

  return version;
}

// safety check so we don't go overwriting accidentally
function checkSymlink(stat) {
  if (!stat.isSymbolicLink()) {
    console.log(
      "Current version (%s) is not a symlink. You may want to copy it into %s.",
      DENO_PATH,
      DVM_PATH
    );
    process.exit(1);
  }
}

// make the symlink
async function link(version = "") {
  let ln = path.join(DVM_PATH, version, DENO_EXECUTOR);
  let stat;

  if (!fs.existsSync(ln)) {
    console.error(
      "deno v%s is not installed. Use `dvm install %s` to install it first.",
      version,
      version
    );
    return process.exit(1);
  }

  if (process.platform === "win32") {
    const admin = await is_admin();

    if (!admin) {
      console.error(
        "You may have to run dvm in a shell (cmd, PowerShell, Git Bash, etc) with elevated (Administrative) privileges to get it to run."
      );
      process.exit(1);
    }
  }

  try {
    stat = fs.lstatSync(DENO_PATH);
    checkSymlink(stat);
  } catch (e) {} /* eslint-disable-line no-empty */

  if (stat) {
    fs.unlinkSync(DENO_PATH);
  }

  fs.symlinkSync(ln, DENO_PATH, "file");
  console.log("now use %s", version);
}

/**
 *
 * @param {string} version
 * @param {string?} registry
 *
 * @returns {string} url
 */
function get_download_url(version, registry = "denocn") {
  const url_prefix = registries[registry].registry;
  let name;

  if (process.platform === "win32") {
    name = "deno_win_x64.zip";
  } else if (process.platform === "darwin") {
    name = "deno_osx_x64.gz";
  } else {
    name = "deno_linux_x64.gz";
  }

  return `${url_prefix}/v${version}/${name}`;
}

/**
 * @param {string} url
 *
 * @returns {Promise}
 */
function download(version, registry = "denocn") {
  let url = get_download_url(version, registry);
  let downloaded_file;

  return Promise.resolve()
    .then(function() {
      let tmpdir = os.tmpdir();
      let fileName = url.split("/").pop();
      downloaded_file = path.join(tmpdir, fileName);

      if (fs.existsSync(downloaded_file)) {
        fs.unlinkSync(downloaded_file);
        return verify_checksum();
      }
      return false;
    })
    .then(function(verified) {
      if (verified) {
        return downloaded_file;
      }

      return request_binary(url, downloaded_file);
    });
}

/**
 * @param {string} file
 * @param {string} checksum
 *
 * @returns {boolean}
 */
/* eslint-disable-next-line no-unused-vars */
function verify_checksum(file, checksum) {
  return false;
}

function request_binary(url, filePath) {
  const writePath = filePath + "-download-" + Date.now();

  return new Promise((resolve, reject) => {
    request(url)
      .on("response", function(response) {
        if (response.statusCode === 404) {
          reject("Deno v%s is not yet released or available.");
          // console.error("Deno v%s is not yet released or available.");
          return;
        }

        const len = parseInt(response.headers["content-length"], 10);

        const writeStream = fs.createWriteStream(writePath);

        // Start the install.
        console.log("Downloading from ", url);

        const bar = new ProgressBar("  downloading [:bar] :percent :etas", {
          complete: "=",
          incomplete: " ",
          width: 20,
          total: len
        });

        response.on("data", function(chunk) {
          writeStream.write(chunk);
          bar.tick(chunk.length);
        });

        response.on("end", function() {
          writeStream.end();
        });

        writeStream.on("finish", function() {
          fs.renameSync(writePath, filePath);
          resolve(filePath);
        });
      })
      .on("error", function(error) {
        handleRequestError(error);
      });
  });
}

function handleRequestError(error) {
  if (error && error.stack) {
    console.error(
      "Error making request.\n" +
        error.stack +
        "\n\n" +
        "Please report this full log at https://github.com/justjavac/dvm"
    );
    process.exit(1);
  } else {
    console.error(
      "Something unexpected happened, please report this full " +
        "log at https://github.com/justjavac/dvm"
    );
    process.exit(1);
  }
}

function extractDownload(filePath, extractedPath) {
  if (fs.existsSync(extractedPath)) {
    fs.rmdirSync(extractedPath);
  }

  fs.mkdirSync(extractedPath, "0777");
  fs.chmodSync(extractedPath, "0777");

  let uncompresser;
  let dest;
  let source = path.resolve(filePath);

  // Note:
  //
  // tar, tgz and zip have the same uncompressing API as above:
  // destination should be a path of a directory,
  // while that of gzip is slightly different:
  // destination must be a file or filestream.
  //
  // https://github.com/node-modules/compressing#uncompress-a-file
  if (filePath.substr(-4) === ".zip") {
    console.log("Extracting zip contents");
    uncompresser = compressing.zip;
    dest = extractedPath;
  } else if (filePath.substr(-3) === ".gz") {
    console.log("Extracting gz contents");
    uncompresser = compressing.gzip;
    dest = path.join(extractedPath, DENO_EXECUTOR);
  }

  return uncompresser
    .uncompress(source, dest)
    .then(() => {
      fs.chmodSync(dest, "0777");
      return extractedPath;
    })
    .catch(err => {
      console.error("Error extracting archive");
      fs.rmdirSync(extractedPath);
      console.error(err);
    });
}
