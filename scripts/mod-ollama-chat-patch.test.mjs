import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, posix as posixPath, win32 as win32Path } from "node:path";
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
  "src/mod-ollama-chat_api.cpp",
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
  "src/mod-ollama-chat_api.cpp",
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

function mergeWindowsEnvironment(base, output) {
  const environment = { ...base };
  for (const line of output.split(/\r?\n/)) {
    const separator = line.indexOf("=");
    if (separator <= 0) continue;
    const name = line.slice(0, separator);
    const value = line.slice(separator + 1);
    for (const existing of Object.keys(environment))
      if (existing.toUpperCase() === name.toUpperCase()) delete environment[existing];
    environment[name] = value;
  }
  return environment;
}

function resolveMsvcEnvironment() {
  const available = spawnSync("cl.exe", ["/nologo", "/?"], { stdio: "ignore" });
  if (!available.error && available.status === 0) return process.env;

  const programFiles =
    process.env["ProgramFiles(x86)"] || process.env.ProgramFiles;
  assert.ok(programFiles, "MSVC unavailable: Program Files is not defined");
  const vswhere = join(
    programFiles,
    "Microsoft Visual Studio",
    "Installer",
    "vswhere.exe",
  );
  let installation;
  try {
    installation = execFileSync(
      vswhere,
      [
        "-latest",
        "-products",
        "*",
        "-requires",
        "Microsoft.VisualStudio.Component.VC.Tools.x86.x64",
        "-property",
        "installationPath",
      ],
      { encoding: "utf8", stdio: ["ignore", "pipe", "pipe"] },
    ).trim();
  } catch (error) {
    throw new Error("MSVC unavailable: vswhere could not locate C++ Build Tools", {
      cause: error,
    });
  }
  assert.ok(
    installation,
    "MSVC unavailable: no C++ Build Tools installation found",
  );
  const developerCommand = join(
    installation,
    "Common7",
    "Tools",
    "VsDevCmd.bat",
  );
  const output = execFileSync(
    "cmd.exe",
    [
      "/d",
      "/s",
      "/c",
      `call "${developerCommand}" -no_logo -arch=x64 -host_arch=x64 >nul && set`,
    ],
    {
      encoding: "utf8",
      stdio: ["ignore", "pipe", "pipe"],
      // This argument already uses cmd.exe quoting, not Windows argv escaping.
      windowsVerbatimArguments: true,
    },
  );
  return mergeWindowsEnvironment(process.env, output);
}

function backoffCompilerInvocation(platform, directory, testSource) {
  const windows = platform === "win32";
  const path = windows ? win32Path : posixPath;
  const executable = path.join(
    directory,
    windows ? "backoff-policy-test.exe" : "backoff-policy-test",
  );
  if (windows) {
    return {
      command: "cl.exe",
      executable,
      args: [
        "/nologo",
        "/std:c++17",
        "/W4",
        "/WX",
        "/EHsc",
        "/permissive-",
        "/utf-8",
        `/I${directory}`,
        testSource,
        `/Fe${executable}`,
        `/Fo${path.join(directory, "backoff-policy-test.obj")}`,
        "/link",
        `/PDB:${path.join(directory, "backoff-policy-test.pdb")}`,
      ],
    };
  }
  return {
    command: process.env.CXX || "c++",
    executable,
    args: [
      "-std=c++17", "-Wall", "-Wextra", "-Werror", "-pedantic",
      "-I", directory, testSource, "-o", executable,
    ],
  };
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

  // All request kinds share one generation circuit in QueryOllama. The pure
  // monotonic-clock policy is bounded, admits one cooldown probe, and ignores
  // completions from an earlier circuit/config generation.
  assert.match(patch, /class RealmBoxOllamaBackoff/);
  assert.match(patch, /FailureThreshold = 3/);
  assert.match(patch, /InitialCooldownMs = 5000/);
  assert.match(patch, /MaximumCooldownMs = 60000/);
  assert.match(patch, /permit\.generation != generation_/);
  assert.match(patch, /probeInFlight_ \|\| deadlineSaturated_/);
  assert.match(patch, /permit = g_backoff\.Acquire\(BackoffNowMs\(\)\)/);
  assert.match(patch, /BackoffCompletion backoff\{ permit \}/);
  assert.match(patch, /backoff\.successful = result\.ok && !result\.text\.empty\(\)/);
  assert.match(patch, /g_backoff\.Reset\(\)/);
  assert.doesNotMatch(patch, /^\+.*(?:sleep_for|sleep_until|std::thread\s*\()/m);

  // The helper snapshots every patched source, verifies the patch allowlist,
  // and restores exact bytes on check-only or failure after CRLF normalization.
  assert.match(helper, /canonicalStats\.ino !== repositoryStats\.ino/);
  assert.match(helper, /assertPatchTargets\(\);/);
  assert.match(helper, /return PATCHED_FILES\.map/);
  assert.match(helper, /normalizePinnedSourceLineEndings\(sourceDirectory\);/);
  assert.match(helper, /restorePinnedSourceSnapshots\(sourceDirectory, snapshots\);/);
});

test("the exact patched C++ backoff policy recovers, saturates, and rejects stale completions", () => {
  const patch = readFileSync(patchPath, "utf8");
  const addedSource = patch.split("\n")
    .filter((line) => line.startsWith("+") && !line.startsWith("+++"))
    .map((line) => line.slice(1))
    .join("\n");
  const policy = addedSource.match(
    /\/\/ REALMBOX_BACKOFF_POLICY_BEGIN\n([\s\S]*?)\/\/ REALMBOX_BACKOFF_POLICY_END/,
  );
  assert.ok(policy, "missing standalone policy markers in the canonical patch");
  const directory = mkdtempSync(join(tmpdir(), "realmbox-ollama-backoff-"));
  const testSource = fileURLToPath(
    new URL("./mod-ollama-backoff-policy.test.cpp", import.meta.url),
  );
  const invocation = backoffCompilerInvocation(
    process.platform,
    directory,
    testSource,
  );
  try {
    writeFileSync(
      join(directory, "RealmBoxOllamaBackoff.h"),
      `#pragma once\n#include <cstdint>\n#include <limits>\n${policy[1]}`,
    );
    execFileSync(invocation.command, invocation.args, {
      cwd: directory,
      env: process.platform === "win32" ? resolveMsvcEnvironment() : process.env,
      stdio: "pipe",
      timeout: 30_000,
    });
    execFileSync(invocation.executable, [], {
      cwd: directory,
      stdio: "pipe",
      timeout: 10_000,
    });
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("the backoff harness selects native compiler flags and isolated output paths", () => {
  const directory = "C:\\RealmBox Test\\tmp";
  const source = "C:\\RealmBox Source\\policy.test.cpp";
  const windows = backoffCompilerInvocation("win32", directory, source);
  assert.equal(windows.command, "cl.exe");
  assert.equal(
    windows.executable,
    win32Path.join(directory, "backoff-policy-test.exe"),
  );
  assert.ok(windows.args.includes("/std:c++17"));
  assert.ok(windows.args.includes("/W4"));
  assert.ok(windows.args.includes("/WX"));
  assert.ok(windows.args.includes("/EHsc"));
  assert.ok(windows.args.includes("/permissive-"));
  assert.ok(windows.args.includes(`/I${directory}`));
  assert.ok(windows.args.includes(`/Fe${windows.executable}`));
  assert.ok(
    windows.args.includes(
      `/Fo${win32Path.join(directory, "backoff-policy-test.obj")}`,
    ),
  );
  assert.ok(
    windows.args.includes(
      `/PDB:${win32Path.join(directory, "backoff-policy-test.pdb")}`,
    ),
  );
  assert.equal(windows.args.filter((argument) => argument === source).length, 1);
  assert.ok(windows.executable.endsWith(".exe"));

  const posix = backoffCompilerInvocation(
    "darwin",
    "/tmp/realm box",
    "/src/policy.cpp",
  );
  assert.equal(posix.command, process.env.CXX || "c++");
  assert.ok(posix.args.includes("-std=c++17"));
  assert.ok(posix.args.includes("-pedantic"));
  assert.ok(posix.args.includes("-o"));
  assert.ok(!posix.executable.endsWith(".exe"));

  const merged = mergeWindowsEnvironment(
    { PATH: "old", KEEP: "yes" },
    "Path=C:\\MSVC\\bin\r\nCOMPLEX=left=right\r\n",
  );
  assert.equal(merged.Path, "C:\\MSVC\\bin");
  assert.equal(merged.COMPLEX, "left=right");
  assert.equal(merged.KEEP, "yes");
  assert.equal(
    Object.keys(merged).filter((name) => name.toUpperCase() === "PATH").length,
    1,
  );
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
