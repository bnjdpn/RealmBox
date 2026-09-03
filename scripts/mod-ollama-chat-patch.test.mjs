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
const expectedPatchedFiles = [
  "src/mod-ollama-chat_command.cpp",
  "src/mod-ollama-chat_dispatch.cpp",
  "src/mod-ollama-chat_dispatch.h",
  "src/mod-ollama-chat_events.cpp",
  "src/mod-ollama-chat_governor.cpp",
  "src/mod-ollama-chat_handler.cpp",
  "src/mod-ollama-chat_random.cpp",
  "src/mod-ollama-chat_random.h",
];
const expectedCrlfFiles = [
  "src/mod-ollama-chat_command.cpp",
  "src/mod-ollama-chat_events.cpp",
  "src/mod-ollama-chat_handler.cpp",
  "src/mod-ollama-chat_random.cpp",
];

function readStringArray(source, name) {
  const declaration = source.match(
    new RegExp(`const ${name} = \\[([\\s\\S]*?)\\];`),
  );
  assert.ok(declaration, `missing ${name}`);
  return [...declaration[1].matchAll(/"([^"]+)"/g)].map((match) => match[1]);
}

test("the mod-ollama patch is pinned, scoped, and preserves priority semantics", () => {
  const patchBytes = readFileSync(patchPath);
  const patch = patchBytes.toString("utf8");
  const helper = readFileSync(scriptPath, "utf8");

  assert.equal(patchBytes.includes(13), false, "the reviewable patch must stay LF-only");
  for (const [index, line] of patch.split("\n").entries())
    assert.doesNotMatch(line, /[ \t]+$/, `patch line ${index + 1} has trailing whitespace`);

  assert.match(patch, new RegExp(`^Upstream-Commit: ${expectedCommit}$`, "m"));
  const patchTargets = [
    ...patch.matchAll(/^diff --git a\/(\S+) b\/(\S+)$/gm),
  ].map((match) => {
    assert.equal(match[1], match[2], "the RealmBox patch must not rename upstream files");
    return match[1];
  });
  assert.deepEqual(patchTargets, expectedPatchedFiles);
  assert.deepEqual(readStringArray(helper, "PATCHED_FILES"), expectedPatchedFiles);
  assert.deepEqual(readStringArray(helper, "CRLF_SOURCE_FILES"), expectedCrlfFiles);

  // One queue slot is reserved for a real player's reply. Player replies are
  // stably prioritized and cannot be rejected by ambient-only governor state
  // at either submission or delivery.
  assert.match(patch, /bool IsHumanReply\(const Task& task\)/);
  assert.match(patch, /TaskType::ChatReply && task\.request\.recordHistory/);
  assert.match(patch, /g_MaxQueueDepth - 1/);
  assert.match(patch, /g_queue\.erase\(evict\)/);
  assert.match(patch, /g_queue\.insert\(firstAmbient, std::move\(task\)\)/);
  assert.match(patch, /!humanReply && Governor_IsRepetitive/);
  assert.match(patch, /!humanReply && !Governor_TryConsumeSend/);
  assert.match(patch, /senderIsBot && !Governor_CanSend/);

  // Reload keeps every player reply but invalidates ambient work from the
  // previous mode, including a completion produced by an in-flight request.
  assert.match(patch, /void OllamaDispatch_OnConfigReload\(\)/);
  assert.match(patch, /OllamaDispatch_OnConfigReload\(\);/);
  assert.match(patch, /std::atomic<uint64_t> g_ambientEpoch/);
  assert.match(patch, /task\.ambientEpoch = g_ambientEpoch\.load/);
  assert.match(patch, /g_ambientEpoch\.fetch_add\(1, std::memory_order_acq_rel\)/);
  assert.match(patch, /g_queue\.erase\(std::remove_if\(g_queue\.begin\(\), g_queue\.end\(\), IsAmbientChat\)/);
  assert.match(patch, /return task\.type == TaskType::ChatReply && !IsHumanReply\(task\)/);
  assert.match(patch, /return !completion\.request\.recordHistory &&/);
  assert.match(patch, /c\.ambientEpoch != g_ambientEpoch\.load/);

  // Party and raid governor state must be isolated by group GUID, never just
  // by zone. The three producers are direct replies, events, and random chat.
  assert.equal((patch.match(/GetGroup\(\)->GetGUID\(\)\.GetCounter\(\)/g) ?? []).length, 3);
  assert.equal((patch.match(/\+#include "Group\.h"/g) ?? []).length, 2);
  assert.match(patch, /key \+= "#p"/);

  // Existing ambient bounds and hot-reload reset remain part of the patch.
  assert.match(patch, /successfulSubmissionsByScope/);
  assert.match(patch, /OllamaRandomChatter_ResetSchedule/);

  // The helper snapshots every patched source, verifies the patch allowlist,
  // and restores exact bytes on check-only or failure after CRLF normalization.
  assert.match(helper, /canonicalStats\.ino !== repositoryStats\.ino/);
  assert.match(helper, /assertPatchTargets\(\);/);
  assert.match(helper, /return PATCHED_FILES\.map/);
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
