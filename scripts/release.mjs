#!/usr/bin/env node
/**
 * リリーススクリプト
 * 使い方: npm run release <version>  例) npm run release 0.0.3
 *
 * 以下のファイルのバージョンを一括更新し、git commit → tag → push する:
 *   - package.json
 *   - src-tauri/Cargo.toml
 *   - src-tauri/tauri.conf.json
 */

import { readFileSync, writeFileSync } from "fs";
import { execSync } from "child_process";
import { resolve, dirname } from "path";
import { fileURLToPath } from "url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const root = resolve(__dirname, "..");

const rawVersion = process.argv[2];
if (!rawVersion) {
  console.error("Usage: npm run release <version>  (例: 0.0.3 または v0.0.3)");
  process.exit(1);
}

const version = rawVersion.replace(/^v/, "");
const tag = `v${version}`;

// --- package.json --------------------------------------------------------
const pkgPath = resolve(root, "package.json");
let pkgContent = readFileSync(pkgPath, "utf8");
pkgContent = pkgContent.replace(/"version": ".*?"/, `"version": "${version}"`);
writeFileSync(pkgPath, pkgContent);
console.log(`  package.json        → ${version}`);

// --- src-tauri/Cargo.toml ------------------------------------------------
// [package] セクション内の `version = "..."` のみ置換 (依存クレートは対象外)
const cargoPath = resolve(root, "src-tauri/Cargo.toml");
let cargoContent = readFileSync(cargoPath, "utf8");
cargoContent = cargoContent.replace(
  /^version = ".*?"/m,
  `version = "${version}"`
);
writeFileSync(cargoPath, cargoContent);
console.log(`  src-tauri/Cargo.toml → ${version}`);

// --- src-tauri/tauri.conf.json -------------------------------------------
const tauriConfPath = resolve(root, "src-tauri/tauri.conf.json");
let tauriConfContent = readFileSync(tauriConfPath, "utf8");
tauriConfContent = tauriConfContent.replace(
  /"version": ".*?"/,
  `"version": "${version}"`
);
writeFileSync(tauriConfPath, tauriConfContent);
console.log(`  src-tauri/tauri.conf.json → ${version}`);

// --- git -----------------------------------------------------------------
console.log(`\nGit: commit, tag ${tag}, push...`);
execSync(
  "git add package.json src-tauri/Cargo.toml src-tauri/tauri.conf.json",
  { cwd: root, stdio: "inherit" }
);
execSync(`git commit -m "chore: bump version to ${tag}"`, {
  cwd: root,
  stdio: "inherit",
});
execSync(`git tag ${tag}`, { cwd: root, stdio: "inherit" });
execSync(`git push origin HEAD ${tag}`, { cwd: root, stdio: "inherit" });

console.log(`\nReleased ${tag}`);
