#!/usr/bin/env node
"use strict";

// This launcher finds the prebuilt sokf binary for this platform and runs it.
// The binary receives the launcher's arguments and uses the launcher's standard
// input, output, and error. When the binary exits, the launcher exits with the
// same code.

const { spawnSync } = require("node:child_process");
const { resolveBinary, exitCode } = require("../lib/binary");

let binary;
try {
  binary = resolveBinary(process.platform, process.arch);
} catch (err) {
  process.stderr.write(`${err.message}\n`);
  process.exit(1);
}

const result = spawnSync(binary, process.argv.slice(2), { stdio: "inherit" });

if (result.error) {
  process.stderr.write(`failed to run sokf: ${result.error.message}\n`);
  process.exit(1);
}

process.exit(exitCode(result));
