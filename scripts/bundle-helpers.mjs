#!/usr/bin/env node
/**
 * Copy Swift helpers next to Waypoint so OCR + camera clips work without
 * Homebrew ffmpeg/ollama on the end-user Mac. The desktop talks to the
 * Waypoint API for coach/LLM; these binaries are Apple frameworks only.
 */
import { copyFileSync, chmodSync, existsSync, mkdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const profile = (process.argv[2] || "debug").replace(/^--/, "");
const helpers = ["waypoint-ocr", "waypoint-encode-clip", "waypoint-face-detect"];
const srcDir = join(root, "src-tauri", "bin");
const appMacos = join(
  root,
  "src-tauri",
  "target",
  profile,
  "bundle",
  "macos",
  "Waypoint.app",
  "Contents",
  "MacOS",
);

if (!existsSync(appMacos)) {
  console.error(`bundle-helpers: missing ${appMacos}`);
  process.exit(1);
}

mkdirSync(appMacos, { recursive: true });
for (const name of helpers) {
  const from = join(srcDir, name);
  const to = join(appMacos, name);
  if (!existsSync(from)) {
    console.error(`bundle-helpers: missing helper ${from} (rebuild src-tauri first)`);
    process.exit(1);
  }
  copyFileSync(from, to);
  chmodSync(to, 0o755);
  console.log(`bundled ${name} → ${to}`);
}
