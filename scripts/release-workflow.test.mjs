import test from "node:test";
import assert from "node:assert/strict";
import { readFileSync, mkdtempSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";

const release = readFileSync(new URL("../.github/workflows/release.yml", import.meta.url), "utf8");
const server = readFileSync(new URL("../.github/workflows/server-images.yml", import.meta.url), "utf8");
const job = (yaml, name) => {
  const content = yaml.split(`\n  ${name}:\n`)[1];
  assert.ok(content, `missing job ${name}`);
  return content.split(/\n  [\w-]+:\n/)[0];
};
const variables = ["REALMBOX_AUTH_SERVER_IMAGE", "REALMBOX_WORLD_SERVER_IMAGE", "REALMBOX_DB_IMPORT_IMAGE", "REALMBOX_TOOLS_IMAGE"];
const outputs = ["authserver", "worldserver", "db_import", "tools"];

test("release builds fresh images for the resolved tag before either installer, with no repository-variable fallback", () => {
  assert.doesNotMatch(release, /vars\.REALMBOX_/);
  const images = job(release, "server-images");
  assert.match(images, /needs: validate/);
  assert.match(images, /uses: \.\/\.github\/workflows\/server-images.yml/);
  assert.match(images, /source_ref: \$\{\{ needs.validate.outputs.source_sha \}\}/);
  assert.match(images, /publish: true/);
  assert.match(images, /build_bundles: false/);
  for (const name of ["macos-arm64", "windows-x64"]) {
    const installer = job(release, name);
    assert.match(installer, /needs: \[validate, server-images, image-lock\]/);
    assert.ok(installer.includes("ref: ${{ needs.validate.outputs.source_sha }}"));
    variables.forEach((variable, index) => {
      assert.ok(installer.includes(`${variable}: \${{ needs.server-images.outputs.${outputs[index]} }}`));
    });
  }
});

test("server image names isolate source, run and attempt instead of reusing upstream-only tags", () => {
  assert.match(server, /workflow_call:/);
  assert.ok(server.includes("SOURCE_SHA: ${{ inputs.source_ref || github.sha }}"));
  assert.ok(server.includes("BUILD_ID: ${{ github.run_id }}-${{ github.run_attempt }}"));
  assert.ok(server.includes('test "$(git rev-parse HEAD)" = "$SOURCE_SHA"'));
  assert.ok(server.includes('--tag "$registry_root/server-$target:rb-$SOURCE_SHA-$BUILD_ID-$SUFFIX"'));
  for (const arch of ["amd64", "arm64"]) {
    assert.ok(server.includes(`"$registry_root/server-$target:rb-$SOURCE_SHA-$BUILD_ID-linux-${arch}"`));
  }
  assert.doesNotMatch(server, /server-\$target:\$SERVER_COMMIT/);
  outputs.forEach(output => assert.ok(server.includes(`value: \${{ jobs.manifest.outputs.${output} }}`)));
  for (const name of ["bundle-macos-arm64", "bundle-windows-x64"]) {
    assert.match(job(server, name), /if: inputs.publish && inputs.build_bundles/);
  }
});

test("manual recovery preserves the tag and uploads provenance only after all build jobs succeed", () => {
  assert.match(release, /workflow_dispatch:\n    inputs:\n      tag:/);
  assert.ok(job(release, "validate").includes("ref: refs/tags/${{ env.RELEASE_TAG }}"));
  assert.match(release, /cancel-in-progress: false/);
  const publish = job(release, "github-release");
  assert.match(publish, /needs: \[validate, macos-arm64, windows-x64, launcher-screenshots\]/);
  assert.ok(publish.includes('test "$(git rev-parse \'FETCH_HEAD^{commit}\')" = "$SOURCE_SHA"'));
  assert.ok(publish.includes("dist/release-provenance/* SHA256SUMS.txt"));
  assert.doesNotMatch(publish, /always\(\)|continue-on-error|git push.*force/);
});

test("the exact image-lock shell rejects absent, mutable, foreign-source and foreign-run references", () => {
  const block = job(release, "image-lock").split("        run: |\n")[1].split("      - uses:")[0];
  const script = block.split("\n").map(line => line.replace(/^          /, "")).join("\n");
  const source = "a".repeat(40);
  const digest = "b".repeat(64);
  const good = `ghcr.io/bnjdpn/realmbox/server-worldserver:rb-${source}-123-1@sha256:${digest}`;
  const cases = [
    [good, true],
    ["", false],
    [good.split("@")[0], false],
    [good.replace(source, "c".repeat(40)), false],
    [good.replace("-123-1@", "-456-1@"), false],
    [good.replace(digest, "bad"), false],
  ];
  for (const [reference, succeeds] of cases) {
    const directory = mkdtempSync(join(tmpdir(), "realmbox-release-lock-"));
    try {
      const env = { ...process.env, SOURCE_SHA: source, GITHUB_RUN_ID: "123" };
      variables.forEach(variable => { env[variable] = good; });
      env.REALMBOX_WORLD_SERVER_IMAGE = reference;
      const bash = process.platform === "win32" ? join(process.env.ProgramFiles ?? "C:\\Program Files", "Git", "bin", "bash.exe") : "bash";
      const result = spawnSync(bash, ["-c", script], { cwd: directory, env, encoding: "utf8" });
      assert.ifError(result.error);
      assert.equal(result.status === 0, succeeds, `${reference}: ${result.stderr}`);
      if (succeeds) {
        assert.equal(readFileSync(join(directory, "release-source.txt"), "utf8"), `${source}\n`);
        assert.equal(readFileSync(join(directory, "release-images.env"), "utf8").trim().split("\n").length, 4);
      }
    } finally {
      rmSync(directory, { recursive: true, force: true });
    }
  }
});
