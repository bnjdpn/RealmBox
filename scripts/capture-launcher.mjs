import { mkdir } from "node:fs/promises";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { chromium } from "playwright";

const root = fileURLToPath(new URL("..", import.meta.url));
const output = fileURLToPath(new URL("../site/public/assets", import.meta.url));
const origin = "http://127.0.0.1:4173";
const server = spawn("pnpm", ["--dir", "apps/desktop", "dev", "--host", "127.0.0.1", "--port", "4173", "--strictPort"], {
  cwd: root,
  stdio: ["ignore", "pipe", "pipe"],
});

async function waitForServer() {
  for (let attempt = 0; attempt < 80; attempt += 1) {
    if (server.exitCode !== null) throw new Error(`Vite stopped with exit code ${server.exitCode}`);
    try {
      const response = await fetch(origin);
      if (response.ok) return;
    } catch {}
    await new Promise((resolve) => setTimeout(resolve, 250));
  }
  throw new Error("Vite did not become ready");
}

async function capture(browser, { name, state, language, panel }) {
  const page = await browser.newPage({ viewport: { width: 1024, height: 640 }, deviceScaleFactor: 1 });
  await page.addInitScript((selectedLanguage) => localStorage.setItem("realmbox-language", selectedLanguage), language);
  await page.goto(`${origin}/?previewState=${state}`, { waitUntil: "networkidle" });
  if (panel) {
    await page.getByRole("button", { name: language === "fr" ? "Réglages" : "Settings" }).click();
    await page.getByRole("button", { name: panel === "companions" ? (language === "fr" ? /Compagnons/ : /Companions/) : (language === "fr" ? /Dialogues/ : /Dialogue/) }).click();
  }
  await page.screenshot({ path: `${output}/${name}`, type: "webp", quality: 88 });
  await page.close();
}

let browser;
try {
  await mkdir(output, { recursive: true });
  await waitForServer();
  browser = await chromium.launch({ headless: true });
  await capture(browser, { name: "launcher-ready-fr.webp", state: "ready", language: "fr" });
  await capture(browser, { name: "launcher-ready-en.webp", state: "ready", language: "en" });
  await capture(browser, { name: "launcher-installing-fr.webp", state: "installing", language: "fr" });
  await capture(browser, { name: "launcher-companions-fr.webp", state: "ready", language: "fr", panel: "companions" });
  await capture(browser, { name: "launcher-companions-en.webp", state: "ready", language: "en", panel: "companions" });
  await capture(browser, { name: "launcher-dialogues-en.webp", state: "ready", language: "en", panel: "dialogues" });
} finally {
  await browser?.close();
  server.kill("SIGTERM");
}
