import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

const expectedCommit = "a9d14b0b8955be136e657ac168dd255f5281a535";
const patchPath = fileURLToPath(
  new URL("../patches/mod-ollama-chat-realmbox.patch", import.meta.url),
);
const scriptPath = fileURLToPath(
  new URL("./apply-pinned-mod-ollama-patch.mjs", import.meta.url),
);

test("the mod-ollama patch is tied to the immutable upstream commit", () => {
  const patch = readFileSync(patchPath, "utf8");
  const helper = readFileSync(scriptPath, "utf8");
  assert.match(patch, new RegExp(`^Upstream-Commit: ${expectedCommit}$`, "m"));
  assert.match(patch, /successfulSubmissionsByScope/);
  assert.match(patch, /OllamaRandomChatter_ResetSchedule/);
  assert.match(helper, /canonicalStats\.ino !== repositoryStats\.ino/);
  assert.match(helper, /normalizePinnedSourceLineEndings\(sourceDirectory\);/);
  assert.match(helper, /restorePinnedSourceSnapshots\(sourceDirectory, snapshots\);/);
});

test("the patch helper fails closed before touching a different checkout", () => {
  const checkout = mkdtempSync(join(tmpdir(), "realmbox-ollama-wrong-pin-"));
  try {
    execFileSync("git", ["init", "--quiet", checkout]);
    writeFileSync(join(checkout, "README.md"), "not the pinned module\n");
    execFileSync("git", ["-C", checkout, "add", "README.md"]);
    execFileSync(
      "git",
      [
        "-C",
        checkout,
        "-c",
        "user.name=RealmBox Tests",
        "-c",
        "user.email=tests@realmbox.invalid",
        "commit",
        "--quiet",
        "-m",
        "fixture",
      ],
    );

    const before = execFileSync("git", ["-C", checkout, "rev-parse", "HEAD"], {
      encoding: "utf8",
    }).trim();
    const result = spawnSync(process.execPath, [scriptPath, checkout, "--check"], {
      encoding: "utf8",
    });

    assert.notEqual(result.status, 0);
    assert.match(result.stderr, new RegExp(`expected pinned commit ${expectedCommit}`));
    assert.equal(
      execFileSync("git", ["-C", checkout, "rev-parse", "HEAD"], {
        encoding: "utf8",
      }).trim(),
      before,
    );
    assert.equal(
      execFileSync("git", ["-C", checkout, "status", "--porcelain"], {
        encoding: "utf8",
      }),
      "",
    );
  } finally {
    rmSync(checkout, { recursive: true, force: true });
  }
});
