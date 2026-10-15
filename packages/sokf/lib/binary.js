"use strict";

// This module maps the host platform to its prebuilt binary package and resolves
// the path of the binary. Its functions take the platform, the architecture, and
// `requireResolve` as parameters, so the tests can run them without the platform
// packages installed.

const PACKAGES = {
  "linux x64": "@six5536/sokf-linux-x64",
  "linux arm64": "@six5536/sokf-linux-arm64",
  "darwin x64": "@six5536/sokf-darwin-x64",
  "darwin arm64": "@six5536/sokf-darwin-arm64",
  "win32 x64": "@six5536/sokf-win32-x64",
};

const SUPPORTED = Object.keys(PACKAGES)
  .map((k) => k.replace(" ", "-"))
  .join(", ");

/**
 * `selectPackage` returns the name of the platform package for a platform and
 * an architecture, or null when no package supports them.
 */
function selectPackage(platform, arch) {
  return PACKAGES[`${platform} ${arch}`] || null;
}

/** `binaryName` returns the file name of the binary inside a platform package. */
function binaryName(platform) {
  return platform === "win32" ? "sokf.exe" : "sokf";
}

// sokf:begin resolve-binary
/**
 * `resolveBinary` returns the absolute path to the prebuilt binary for a
 * platform and an architecture. It throws an error when the platform has no
 * binary, or when the platform package isn't installed, and each error tells
 * the user how to fix the problem. `requireResolve` defaults to Node's own
 * `require.resolve`.
 */
function resolveBinary(platform, arch, requireResolve = require.resolve) {
  const pkg = selectPackage(platform, arch);
  if (!pkg) {
    throw new Error(
      `There is no prebuilt sokf binary for ${platform}-${arch}.\n` +
        `sokf has prebuilt binaries for ${SUPPORTED}.\n` +
        `Install from source instead: cargo install sokf`,
    );
  }
  try {
    return requireResolve(`${pkg}/bin/${binaryName(platform)}`);
  } catch {
    throw new Error(
      `The sokf platform package "${pkg}" is not installed.\n` +
        `npm most likely skipped the optional dependencies when it installed sokf.\n` +
        `Reinstall sokf, or build from source: cargo install sokf`,
    );
  }
}
// sokf:end resolve-binary

// sokf:begin exit-code
/**
 * `exitCode` maps a `spawnSync` result to the exit code of this process.
 *
 * When a signal stops the child, the result has `status === null` and the
 * signal's name, such as `"SIGINT"`. Shells report a process stopped by a
 * signal as 128 plus the signal's number. `exitCode` does the same, so a run
 * stopped with Ctrl-C exits with 130 rather than a misleading 1.
 * `signals` defaults to Node's own table of signal numbers, and the tests pass
 * in their own.
 */
function exitCode(result, signals = require("node:os").constants.signals) {
  if (result.signal) {
    const signum = signals[result.signal];
    if (signum) {
      return 128 + signum;
    }
  }
  return result.status === null ? 1 : result.status;
}
// sokf:end exit-code

module.exports = {
  selectPackage,
  binaryName,
  resolveBinary,
  exitCode,
  PACKAGES,
  SUPPORTED,
};
