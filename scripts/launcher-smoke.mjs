#!/usr/bin/env node
// This script smoke-tests the npm launcher against the real packed artifacts.
// It runs `npm pack` on the launcher and on the host's platform package,
// extracts both into a temporary node_modules, and runs the launcher end to
// end. The test checks the packages' `files` manifests, so a binary left out
// of a tarball makes the test fail. It also checks that the launcher resolves the
// binary, spawns it, and forwards the binary's exit code.
//
// The launcher selects the platform package by the host's platform and
// architecture, so the script can test only the host's own package. Before the
// script runs, the binary must be in `packages/PKG/bin/`, where PKG is the
// directory of the host's platform package, such as sokf-linux-x64. Release
// continuous integration (CI) stages the binary there after the build, and on
// your own machine you copy a release build there first.

import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, mkdtempSync, renameSync, rmSync } from "node:fs";
import { createRequire } from "node:module";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";

const require_ = createRequire(import.meta.url);
const { selectPackage, binaryName } = require_("../packages/sokf/lib/binary.js");

function fail(message) {
  console.error(`launcher-smoke: ${message}`);
  process.exit(1);
}

/** `run` runs a command, and exits if the command cannot start. */
function run(cmd, args, opts = {}) {
  // On Windows npm is npm.cmd, which only a shell resolves.
  const shell = process.platform === "win32" && cmd === "npm";
  const r = spawnSync(cmd, args, { encoding: "utf8", shell, ...opts });
  if (r.error) {
    fail(`failed to run ${cmd}: ${r.error.message}`);
  }
  return r;
}

const pkgName = selectPackage(process.platform, process.arch);
if (!pkgName) {
  fail(`unsupported host ${process.platform}-${process.arch}`);
}
const pkgDir = join("packages", pkgName.replace("@six5536/", ""));
const binName = binaryName(process.platform);
if (!existsSync(join(pkgDir, "bin", binName))) {
  fail(
    `no staged binary at ${join(pkgDir, "bin", binName)}: ` +
      `copy a release build there first, as the "Stage binary" step of release.yml does`,
  );
}

const base = mkdtempSync(join(tmpdir(), "sokf-launcher-smoke-"));
try {
  // Pack both packages as npm would publish them, with their `files` manifests applied.
  const pack = run("npm", ["pack", "./packages/sokf", `./${pkgDir}`, "--pack-destination", base]);
  if (pack.status !== 0) {
    console.error(pack.stdout);
    console.error(pack.stderr);
    fail("npm pack failed");
  }
  const tarballs = pack.stdout.trim().split("\n").slice(-2);

  // Extract each tarball, whose root directory is `package/`, into a node_modules layout.
  const nm = join(base, "node_modules");
  const dests = [join(nm, "sokf"), join(nm, pkgName)];
  for (const [i, tgz] of tarballs.entries()) {
    const scratch = join(base, `x${i}`);
    mkdirSync(scratch, { recursive: true });
    // The paths are relative and tar runs in the temporary directory. On the
    // Windows runner, git-bash provides GNU tar, which reads the colon after the
    // drive letter in C:\... as the name of a remote host.
    const tar = run("tar", ["-xzf", tgz, "-C", `x${i}`], { cwd: base });
    if (tar.status !== 0) {
      fail(`tar failed on ${tgz}: ${tar.stderr}`);
    }
    mkdirSync(resolve(dests[i], ".."), { recursive: true });
    renameSync(join(scratch, "package"), dests[i]);
  }

  const packedBinary = join(nm, pkgName, "bin", binName);
  if (!existsSync(packedBinary)) {
    fail(`${pkgName}'s tarball does not contain bin/${binName}: check its "files" manifest`);
  }

  const launcher = join(nm, "sokf", "bin", "sokf.js");
  const version = run(process.execPath, [launcher, "--version"]);
  if (version.status !== 0 || !/^sokf \d+\.\d+\.\d+/.test(version.stdout.trim())) {
    console.error(version.stdout);
    console.error(version.stderr);
    fail(`launcher --version failed (exit code ${version.status})`);
  }

  // The launcher must forward the exit code of the binary. An unknown flag is a
  // usage error to clap, so sokf exits with 2. `info grammar` reads no
  // repository, so sokf exits with 0.
  const bad = run(process.execPath, [launcher, "--definitely-not-a-flag"]);
  if (bad.status !== 2) {
    fail(`the launcher forwarded exit code ${bad.status} for a usage error, but sokf exits with 2 for a usage error`);
  }
  const grammar = run(process.execPath, [launcher, "info", "grammar"]);
  if (grammar.status !== 0 || !grammar.stdout.startsWith("grammar: tramlines\n")) {
    fail(`launcher did not print the schema language (exit code ${grammar.status})`);
  }

  console.log(`launcher-smoke OK: the launcher resolves ${pkgName} and forwards the binary's exit code (${version.stdout.trim()})`);
} finally {
  rmSync(base, { recursive: true, force: true });
}
