#!/usr/bin/env node
// pack-version.mts lists or sets the version of each pack under packs/,
// because each pack is versioned with sokf's tags, so it changes only with a
// release and takes the tool's version.
//
// Usage: node scripts/pack-version.mts list | set <version>

import { readdirSync, readFileSync, writeFileSync } from "node:fs";

const [mode, version] = process.argv.slice(2);
for (const name of readdirSync("packs")) {
  const path = `packs/${name}/pakf.yaml`;
  const text = readFileSync(path, "utf8");
  if (mode === "set") {
    writeFileSync(path, text.replace(/^version: *\S+$/m, `version: ${version}`));
  } else {
    const match = text.match(/^version: *(\S+)$/m);
    if (match === null) throw new TypeError(`no version line in ${path}`);
    console.log(`${path} ${match[1]}`);
  }
}
