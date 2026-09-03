#!/usr/bin/env node

import { execFileSync, spawnSync } from "node:child_process";
import { readFileSync, realpathSync, statSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const PINNED_COMMIT = "a9d14b0b8955be136e657ac168dd255f5281a535";
const PATCH_PATH = fileURLToPath(
  new URL("../patches/mod-ollama-chat-realmbox.patch", import.meta.url),
);
const PATCHED_FILES = [
  "src/mod-ollama-chat_command.cpp",
  "src/mod-ollama-chat_dispatch.cpp",
  "src/mod-ollama-chat_dispatch.h",
  "src/mod-ollama-chat_events.cpp",
  "src/mod-ollama-chat_governor.cpp",
  "src/mod-ollama-chat_handler.cpp",
  "src/mod-ollama-chat_random.cpp",
  "src/mod-ollama-chat_random.h",
];
const CRLF_SOURCE_FILES = [
  "src/mod-ollama-chat_command.cpp",
  "src/mod-ollama-chat_events.cpp",
  "src/mod-ollama-chat_handler.cpp",
  "src/mod-ollama-chat_random.cpp",
];

function fail(message) {
  throw new Error(`RealmBox mod-ollama-chat patch refused: ${message}`);
}

function git(sourceDirectory, args) {
  try {
    return execFileSync("git", ["-C", sourceDirectory, ...args], {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
    }).trim();
  } catch (error) {
    const stderr = error?.stderr?.trim();
    fail(stderr || `git ${args.join(" ")} failed`);
  }
}

function assertPatchTargets() {
  const patch = readFileSync(PATCH_PATH, "utf8");
  const targets = [];
  for (const match of patch.matchAll(/^diff --git a\/(\S+) b\/(\S+)$/gm)) {
    if (match[1] !== match[2])
      fail(`patch renames a source file: ${match[1]} -> ${match[2]}`);
    targets.push(match[1]);
  }

  if (targets.length === 0)
    fail("patch contains no source targets");
  if (
    targets.length !== PATCHED_FILES.length ||
    targets.some((target, index) => target !== PATCHED_FILES[index])
  )
    fail("patch targets do not match the reviewed source allowlist");
}

function assertPinnedCleanTarget(sourceDirectory) {
  let sourceStats;
  try {
    sourceStats = statSync(sourceDirectory);
  } catch {
    fail(`source directory does not exist: ${sourceDirectory}`);
  }
  if (!sourceStats.isDirectory())
    fail(`source path is not a directory: ${sourceDirectory}`);

  const canonicalSource = realpathSync(sourceDirectory);
  const repositoryRoot = realpathSync(git(canonicalSource, ["rev-parse", "--show-toplevel"]));
  const canonicalStats = statSync(canonicalSource);
  const repositoryStats = statSync(repositoryRoot);
  if (
    canonicalStats.dev !== repositoryStats.dev ||
    canonicalStats.ino !== repositoryStats.ino
  )
    fail(`source must be the checkout root, got ${canonicalSource}`);

  const head = git(canonicalSource, ["rev-parse", "HEAD"]);
  if (head !== PINNED_COMMIT)
    fail(`expected pinned commit ${PINNED_COMMIT}, got ${head}`);

  const patchHeader = readFileSync(PATCH_PATH, "utf8").match(/^Upstream-Commit: ([0-9a-f]{40})$/m);
  if (!patchHeader || patchHeader[1] !== PINNED_COMMIT)
    fail("patch metadata does not match the pinned commit");
  assertPatchTargets();

  const dirty = spawnSync(
    "git",
    ["-C", canonicalSource, "diff", "--quiet", "HEAD", "--", ...PATCHED_FILES],
    { encoding: "utf8" },
  );
  if (dirty.error)
    fail(dirty.error.message);
  if (dirty.status !== 0)
    fail("one or more patched source files already contain changes");

  return canonicalSource;
}

function snapshotPinnedSources(sourceDirectory) {
  return PATCHED_FILES.map((relativePath) => ({
    relativePath,
    contents: readFileSync(`${sourceDirectory}/${relativePath}`),
  }));
}

function normalizePinnedSourceLineEndings(sourceDirectory) {
  // The pinned upstream stores these four files with CRLF bytes in Git. A
  // normal LF patch is intentionally kept reviewable in RealmBox, so the
  // helper normalizes the checkout before both `git apply --check` and apply.
  for (const relativePath of CRLF_SOURCE_FILES) {
    const path = `${sourceDirectory}/${relativePath}`;
    const normalized = readFileSync(path, "utf8").replace(/\r\n/g, "\n");
    writeFileSync(path, normalized);
  }
}

function restorePinnedSourceSnapshots(sourceDirectory, snapshots) {
  for (const { relativePath, contents } of snapshots)
    writeFileSync(`${sourceDirectory}/${relativePath}`, contents);
}

function preservePinnedSourceLineEndings(sourceDirectory) {
  for (const relativePath of CRLF_SOURCE_FILES) {
    const path = `${sourceDirectory}/${relativePath}`;
    const normalized = readFileSync(path, "utf8").replace(/\r\n/g, "\n");
    writeFileSync(path, normalized.replace(/\n/g, "\r\n"));
  }
}

function main() {
  const args = process.argv.slice(2);
  const checkOnly = args.at(-1) === "--check";
  if (checkOnly)
    args.pop();
  if (args.length !== 1)
    fail("usage: apply-pinned-mod-ollama-patch.mjs <checkout-root> [--check]");

  const sourceDirectory = assertPinnedCleanTarget(args[0]);
  const snapshots = snapshotPinnedSources(sourceDirectory);
  try {
    normalizePinnedSourceLineEndings(sourceDirectory);
    git(sourceDirectory, ["apply", "--check", PATCH_PATH]);

    if (checkOnly) {
      restorePinnedSourceSnapshots(sourceDirectory, snapshots);
      process.stdout.write(`Patch is applicable to ${PINNED_COMMIT}.\n`);
      return;
    }

    git(sourceDirectory, ["apply", PATCH_PATH]);
    preservePinnedSourceLineEndings(sourceDirectory);
  } catch (error) {
    restorePinnedSourceSnapshots(sourceDirectory, snapshots);
    throw error;
  }
  process.stdout.write(`Applied RealmBox mod-ollama-chat patch to ${PINNED_COMMIT}.\n`);
}

try {
  main();
} catch (error) {
  process.stderr.write(`${error.message}\n`);
  process.exitCode = 1;
}
