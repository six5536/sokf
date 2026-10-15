#!/usr/bin/env node
// verify-version.mjs checks that every place in the tree holds the same
// version: the Cargo workspace, the internal sokf-core pin, every package.json
// under packages/, the launcher's optionalDependencies, and both lockfiles.
//
// Usage: node scripts/verify-version.mjs [EXPECTED_VERSION]
//
// EXPECTED_VERSION is the version that every place must hold, such as 1.2.3,
// with or without a leading v. With no argument, the script checks only that the versions agree with one
// another. With an argument, it also checks that every version equals the
// argument. The release workflow uses the argument to check a tag against the
// tree.

import { readFileSync, readdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";

const expected = process.argv[2]?.replace(/^v/, "");
const root = join(dirname(fileURLToPath(import.meta.url)), "..");

/** `found` holds a { where, version } record for each place where a version is written. */
const found = [];
const problems = [];

const record = (where, version) => {
  if (version === undefined) {
    problems.push(`could not read a version from ${where}`);
    return;
  }
  found.push({ where, version });
};

// --- Cargo.toml: the workspace version and the internal sokf-core pin ------
const cargo = readFileSync(join(root, "Cargo.toml"), "utf8");
record("Cargo.toml [workspace.package] version", cargo.match(/^version = "([^"]*)"$/m)?.[1]);
record(
  "Cargo.toml sokf-core dependency pin",
  cargo.match(/sokf-core = \{ path = "crates\/lib\/sokf-core", version = "([^"]*)" \}/)?.[1],
);

// --- packages/*/package.json and the launcher's optionalDependencies ---------
for (const name of readdirSync(join(root, "packages"))) {
  const path = join(root, "packages", name, "package.json");
  let json;
  try {
    json = JSON.parse(readFileSync(path, "utf8"));
  } catch {
    continue;
  }
  record(`packages/${name}/package.json version`, json.version);
  for (const [dep, range] of Object.entries(json.optionalDependencies ?? {})) {
    if (dep.startsWith("@six5536/sokf-")) {
      record(`packages/${name}/package.json optionalDependencies["${dep}"]`, range);
    }
  }
}

// --- Cargo.lock -------------------------------------------------------------
const cargoLock = readFileSync(join(root, "Cargo.lock"), "utf8");
for (const crate of ["sokf", "sokf-core"]) {
  const re = new RegExp(`name = "${crate}"\\nversion = "([^"]*)"`);
  record(`Cargo.lock ${crate}`, cargoLock.match(re)?.[1]);
}

// --- package-lock.json ------------------------------------------------------
const npmLock = JSON.parse(readFileSync(join(root, "package-lock.json"), "utf8"));
for (const [key, entry] of Object.entries(npmLock.packages ?? {})) {
  if (key.startsWith("packages/") && entry.version) {
    record(`package-lock.json ${key}`, entry.version);
  }
}

// --- Verdict ----------------------------------------------------------------
const versions = [...new Set(found.map((f) => f.version))];
const target = expected ?? versions[0];

if (versions.length !== 1) {
  problems.push(`the versions in the tree disagree: ${versions.join(", ")}`);
}
for (const { where, version } of found) {
  if (version !== target) {
    problems.push(`${where}: found ${version}, expected ${target}`);
  }
}

if (problems.length > 0) {
  console.error("version check failed:");
  for (const p of problems) console.error(`  - ${p}`);
  console.error("\nTo make the versions agree, run `npm run set-version VERSION`, where VERSION is the version to set.");
  process.exit(1);
}

console.log(`all ${found.length} places hold version ${target}`);
