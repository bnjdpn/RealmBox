const supportedLanguages = new Set(["fr", "en"]);
const queryLanguage = new URLSearchParams(window.location.search).get("lang");
const storedLanguage = window.localStorage.getItem("realmbox-site-language");
const browserLanguage = window.navigator.language.toLowerCase().startsWith("fr") ? "fr" : "en";
const releasesUrl = "https://github.com/bnjdpn/RealmBox/releases";

function currentLanguage() {
  return document.documentElement.lang === "en" ? "en" : "fr";
}

function setLanguage(language) {
  const selected = supportedLanguages.has(language) ? language : "fr";
  document.documentElement.lang = selected;
  window.localStorage.setItem("realmbox-site-language", selected);

  document.querySelectorAll("[data-set-language]").forEach((button) => {
    const active = button.dataset.setLanguage === selected;
    button.classList.toggle("active", active);
    button.setAttribute("aria-pressed", String(active));
  });

  document.querySelectorAll("[data-aria-fr][data-aria-en]").forEach((element) => {
    element.setAttribute("aria-label", selected === "fr" ? element.dataset.ariaFr : element.dataset.ariaEn);
  });

  document.title = selected === "fr"
    ? "RealmBox — votre monde 3.3.5a local"
    : "RealmBox — your local 3.3.5a world";

  updateRecommendedOs();
}

function detectedOs() {
  const platform = `${window.navigator.userAgent} ${window.navigator.platform}`.toLowerCase();
  if (platform.includes("mac")) return "macos";
  if (platform.includes("win")) return "windows";
  return null;
}

function updateRecommendedOs() {
  const os = detectedOs();
  document.querySelectorAll("[data-download-os]").forEach((link) => {
    const recommended = link.dataset.downloadOs === os;
    link.classList.toggle("is-recommended", recommended);
    const badge = link.querySelector("[data-os-badge]");
    if (badge) badge.textContent = recommended ? (currentLanguage() === "fr" ? "Pour vous" : "For you") : "";
  });
}

function setReleaseStatus(fr, en) {
  const status = document.querySelector("#release-status");
  if (!status) return;
  status.replaceChildren();
  const frText = document.createElement("span");
  frText.dataset.fr = "";
  frText.textContent = fr;
  const enText = document.createElement("span");
  enText.dataset.en = "";
  enText.textContent = en;
  status.append(frText, enText);
}

async function wireLatestRelease() {
  try {
    const response = await fetch("https://api.github.com/repos/bnjdpn/RealmBox/releases?per_page=20", {
      headers: { Accept: "application/vnd.github+json" },
    });
    if (!response.ok) throw new Error(`GitHub returned ${response.status}`);

    const releases = await response.json();
    const latest = releases.find((release) => !release.draft);
    if (!latest) {
      setReleaseStatus(
        "Aucune release publique n’est encore disponible. La page GitHub s’ouvrira pour vous.",
        "No public release is available yet. The GitHub releases page will open for you.",
      );
      return;
    }

    const assets = Array.isArray(latest.assets) ? latest.assets : [];
    const macAsset = assets.find((asset) => asset.name.toLowerCase().endsWith(".dmg"));
    const windowsAsset = assets.find((asset) => asset.name.toLowerCase().endsWith(".exe"));
    const links = {
      macos: macAsset?.browser_download_url,
      windows: windowsAsset?.browser_download_url,
    };

    document.querySelectorAll("[data-download-os]").forEach((link) => {
      const directUrl = links[link.dataset.downloadOs];
      link.href = directUrl || latest.html_url || releasesUrl;
    });

    setReleaseStatus(
      `Dernière version publique : ${latest.tag_name}${latest.prerelease ? " · préversion" : ""}.`,
      `Latest public version: ${latest.tag_name}${latest.prerelease ? " · preview" : ""}.`,
    );
  } catch {
    setReleaseStatus(
      "La vérification automatique est indisponible. Consultez la page des releases.",
      "Automatic lookup is unavailable. Please check the releases page.",
    );
  }
}

document.querySelectorAll("[data-set-language]").forEach((button) => {
  button.addEventListener("click", () => setLanguage(button.dataset.setLanguage));
});

setLanguage(queryLanguage ?? storedLanguage ?? browserLanguage);
wireLatestRelease();
