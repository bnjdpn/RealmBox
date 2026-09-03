import assert from "node:assert/strict";
import test from "node:test";
import { applyRelease, parseChecksums, selectAssets } from "./generate-site-release-manifest.mjs";

const release = {
  tag_name: "v0.3.0",
  prerelease: true,
  published_at: "2026-09-03T12:00:00Z",
  html_url: "https://example.test/release",
  assets: [
    { name: "RealmBox_0.3.0_aarch64.dmg", browser_download_url: "https://example.test/mac" },
    { name: "RealmBox_0.3.0_x64-setup.exe", browser_download_url: "https://example.test/windows" },
    { name: "SHA256SUMS.txt", browser_download_url: "https://example.test/sums" },
  ],
};

test("selectAssets keeps platform downloads distinct", () => {
  const selected = selectAssets(release);
  assert.equal(selected.macosAppleSilicon.name, "RealmBox_0.3.0_aarch64.dmg");
  assert.equal(selected.windowsX64.name, "RealmBox_0.3.0_x64-setup.exe");
});

test("parseChecksums and applyRelease inject public links without changing qualification", () => {
  const checksums = parseChecksums(`${"a".repeat(64)}  RealmBox_0.3.0_aarch64.dmg\n${"b".repeat(64)} *RealmBox_0.3.0_x64-setup.exe\n`);
  const manifest = {
    platforms: {
      macosAppleSilicon: { status: "qualified", assetUrl: null, assetName: null, sha256: null },
      windowsX64: { status: "experimental", assetUrl: null, assetName: null, sha256: null },
    },
  };
  const result = applyRelease(manifest, release, checksums);
  assert.equal(result.publicRelease.version, "0.3.0");
  assert.equal(result.platforms.macosAppleSilicon.status, "qualified");
  assert.equal(result.platforms.windowsX64.status, "experimental");
  assert.equal(result.platforms.macosAppleSilicon.sha256, "a".repeat(64));
  assert.equal(result.platforms.windowsX64.assetUrl, "https://example.test/windows");
});
