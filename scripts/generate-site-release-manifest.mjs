import { readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";

const manifestPath = fileURLToPath(new URL("../site/public/release-manifest.json", import.meta.url));
const packagePath = fileURLToPath(new URL("../package.json", import.meta.url));

export function selectAssets(release) {
  const assets = release?.assets ?? [];
  return {
    macosAppleSilicon: assets.find((asset) => /\.(dmg)$/i.test(asset.name) && /(arm64|aarch64)/i.test(asset.name)) ?? null,
    windowsX64: assets.find((asset) => /\.(exe)$/i.test(asset.name) && /(x64|x86_64)/i.test(asset.name)) ?? null,
    checksums: assets.find((asset) => /^SHA256SUMS\.txt$/i.test(asset.name)) ?? null,
  };
}

export function parseChecksums(text) {
  return Object.fromEntries(
    text.split(/\r?\n/).flatMap((line) => {
      const match = line.trim().match(/^([a-f0-9]{64})\s+\*?(.+)$/i);
      return match ? [[match[2].trim(), match[1].toLowerCase()]] : [];
    }),
  );
}

export function applyRelease(manifest, release, checksums = {}) {
  const selected = selectAssets(release);
  const next = structuredClone(manifest);
  next.updatedAt = new Date().toISOString().slice(0, 10);
  next.publicRelease = release ? {
    version: release.tag_name.replace(/^v/, ""),
    tag: release.tag_name,
    prerelease: Boolean(release.prerelease),
    publishedAt: release.published_at,
    url: release.html_url,
  } : null;

  for (const key of ["macosAppleSilicon", "windowsX64"]) {
    const asset = selected[key];
    next.platforms[key].assetUrl = asset?.browser_download_url ?? null;
    next.platforms[key].assetName = asset?.name ?? null;
    next.platforms[key].sha256 = asset ? checksums[asset.name] ?? null : null;
  }
  return next;
}

async function githubJson(path, token) {
  const response = await fetch(`https://api.github.com${path}`, {
    headers: {
      Accept: "application/vnd.github+json",
      "User-Agent": "RealmBox-Pages",
      ...(token ? { Authorization: `Bearer ${token}` } : {}),
    },
  });
  if (!response.ok) throw new Error(`GitHub API ${response.status}`);
  return response.json();
}

async function main() {
  const [manifest, packageJson] = await Promise.all([
    readFile(manifestPath, "utf8").then(JSON.parse),
    readFile(packagePath, "utf8").then(JSON.parse),
  ]);
  manifest.productVersion = packageJson.version;
  const repository = process.env.GITHUB_REPOSITORY;
  const token = process.env.GITHUB_TOKEN;
  let release = null;
  let checksums = {};

  if (repository) {
    try {
      const releases = await githubJson(`/repos/${repository}/releases?per_page=20`, token);
      release = releases.find((candidate) => !candidate.draft) ?? null;
      const checksumAsset = selectAssets(release).checksums;
      if (checksumAsset) {
        const response = await fetch(checksumAsset.browser_download_url, {
          headers: token ? { Authorization: `Bearer ${token}` } : {},
        });
        if (!response.ok) throw new Error(`checksum download ${response.status}`);
        checksums = parseChecksums(await response.text());
      }
    } catch (error) {
      console.warn(`Release manifest kept conservative: ${error instanceof Error ? error.message : error}`);
      release = null;
      checksums = {};
    }
  }

  await writeFile(manifestPath, `${JSON.stringify(applyRelease(manifest, release, checksums), null, 2)}\n`);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) await main();
