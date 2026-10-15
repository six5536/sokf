#!/usr/bin/env node
// release-smoke.mjs is a smoke test of a compiled sokf binary. It runs the
// binary and checks what a user sees: the version line, the grammar and the
// formats, a store that `sokf init` creates and `sokf validate` checks, a
// schema that `sokf new` creates, and the exit codes 0, 1, and 2. Release
// continuous integration (CI) runs the script against each built artifact that its runner can execute. To
// run it on your machine, run `cargo build --release -p sokf`, then
// `npm run smoke`.
//
// The script mirrors crates/app/sokf/tests/it/cli.rs on purpose: those tests
// check the code, and this script checks the shipped artifact. When you extend one, extend the other.

import { spawnSync } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";

const bin =
  process.argv[2] ??
  join("target", "release", process.platform === "win32" ? "sokf.exe" : "sokf");

function fail(message) {
  console.error(`release-smoke: ${message}`);
  process.exit(1);
}

if (!existsSync(bin)) {
  fail(`no binary at ${bin}: build it first with cargo build --release -p sokf`);
}

/** run runs the binary, checks its exit status, and returns the spawn result. */
function run(args, expectStatus) {
  const r = spawnSync(bin, args, { encoding: "utf8" });
  if (r.error) {
    fail(`failed to run ${bin}: ${r.error.message}`);
  }
  if (r.status !== expectStatus) {
    console.error(r.stdout);
    console.error(r.stderr);
    fail(`\`sokf ${args.join(" ")}\` exited ${r.status}, expected ${expectStatus}`);
  }
  return r;
}

// --- The version line, which the publish step also checks against the tag ---
const version = run(["--version"], 0).stdout.trim();
if (!/^sokf \d+\.\d+\.\d+/.test(version)) {
  fail(`unexpected --version output: ${version}`);
}

// --- info: the schema language and the embedded formats, which need no repository ---
if (!run(["info", "grammar"], 0).stdout.startsWith("grammar: tramlines\n")) {
  fail("info grammar does not print the schema language");
}
if (!run(["info", "formats", "--layer", "embedded"], 0).stdout.includes("source: embedded")) {
  fail("info formats does not print the embedded registry");
}

// --- `sokf init`, then `sokf validate`: a store that `sokf init` creates
// --- passes (exit 0), and fails after the script adds a broken document (exit 1) ---
const repo = mkdtempSync(join(tmpdir(), "sokf-smoke-"));
try {
  const created = run(["--repo-root", repo, "init", "--scope", "smoke"], 0).stdout;
  if (
    created !==
    "created /knowledge/index.md\ncreated /knowledge/manifest.sokf.yaml\ncreated /knowledge/schemas/index.md\n"
  ) {
    fail(`unexpected init report: ${JSON.stringify(created)}`);
  }
  const clean = run(["--repo-root", repo, "validate"], 0);
  if (clean.stdout !== "0 errors, 0 warnings\n") {
    fail(`unexpected clean report: ${JSON.stringify(clean.stdout)}`);
  }

  // `sokf harness install` writes every part, and a second install reports
  // every part as current.
  const installed = run(["--repo-root", repo, "harness", "install", "claude"], 0).stdout;
  if (
    installed !==
    "claude:\n  created /CLAUDE.md (instructions)\n  created /.claude/skills (skills)\n  created /.claude/settings.json (hooks)\n  created /.claude/settings.json (permissions)\n"
  ) {
    fail(`unexpected harness install report: ${JSON.stringify(installed)}`);
  }
  const again = run(["--repo-root", repo, "harness", "install", "claude"], 0).stdout;
  const parts = again.split("\n").filter((line) => line.startsWith("  "));
  if (parts.length === 0 || !parts.every((line) => line.trimStart().startsWith("current "))) {
    fail(`after a second harness install, not every part is current: ${JSON.stringify(again)}`);
  }

  // `sokf new` creates a schema from the grammar's own template. The schema
  // passes `sokf validate` after its writer deletes the guidance lines, and
  // `sokf validate --fix` writes the index that lists it.
  const made = run(["--repo-root", repo, "new", "Schema", "--id", "schema-note", "--write"], 0).stdout;
  if (made !== "created /knowledge/schemas/schema-note.md\n") {
    fail(`unexpected new report: ${JSON.stringify(made)}`);
  }
  const schema = join(repo, "knowledge", "schemas", "schema-note.md");
  writeFileSync(
    schema,
    readFileSync(schema, "utf8")
      .split("\n")
      .filter((line) => !line.includes("sokf:guidance"))
      .join("\n"),
  );
  run(["--repo-root", repo, "validate", "--fix"], 0);
  if (run(["--repo-root", repo, "validate"], 0).stdout !== "0 errors, 0 warnings\n") {
    fail("the created schema does not validate");
  }

  writeFileSync(
    join(repo, "knowledge", "orphan.md"),
    "---\ntype: Orphan\nid: orphan\n---\n\n# Orphan\n",
  );
  const broken = run(["--repo-root", repo, "validate", "--json"], 1);
  let doc;
  try {
    doc = JSON.parse(broken.stdout);
  } catch (err) {
    fail(`--json output is not valid JSON (${err.message}): ${JSON.stringify(broken.stdout)}`);
  }
  if (!Array.isArray(doc.errors) || doc.errors.length === 0 || !Array.isArray(doc.fixes)) {
    fail(`unexpected --json document: ${broken.stdout}`);
  }
  if (!doc.errors.some((e) => e.path === "/knowledge/orphan.md" && /no schema/.test(e.message))) {
    fail(`the orphan type is not reported: ${broken.stdout}`);
  }
  // The generated index doesn't list the orphan document, so `sokf validate`
  // reports the index as stale.
  if (!doc.errors.some((e) => e.path === "/knowledge/index.md" && /stale/.test(e.message))) {
    fail(`the stale index is not reported: ${broken.stdout}`);
  }
} finally {
  rmSync(repo, { recursive: true, force: true });
}

// --- The man page and the shell completions, which the release archives include ---
const man = run(["man"], 0).stdout;
if (!man.includes(".TH sokf 1") || !man.includes(".SH COMMANDS")) {
  fail("man output is missing its expected sections");
}
if (!run(["completions", "bash"], 0).stdout.includes("_sokf()")) {
  fail("bash completions are missing the _sokf function");
}

// --- Failure paths: sokf exits 2 when the repository is missing and on a usage error ---
const noRepo = run(["--repo-root", join(tmpdir(), "sokf-smoke-definitely-missing"), "validate"], 2);
if (!noRepo.stderr.startsWith("error: ")) {
  fail(`a failed run did not print an error on stderr: ${JSON.stringify(noRepo.stderr)}`);
}
run(["--definitely-not-a-flag"], 2);

console.log(`release-smoke OK: ${bin} (${version}) validates a store, prints its grammar, and exits 0, 1, and 2`);
