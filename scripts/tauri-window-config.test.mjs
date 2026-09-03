import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import test from "node:test";

test("desktop window is the fixed 1024 by 640 launcher surface", async () => {
  const config = JSON.parse(await readFile(new URL("../apps/desktop/src-tauri/tauri.conf.json", import.meta.url), "utf8"));
  const [window] = config.app.windows;

  assert.deepEqual(window, {
    title: "RealmBox",
    width: 1024,
    height: 640,
    minWidth: 1024,
    minHeight: 640,
    maxWidth: 1024,
    maxHeight: 640,
    resizable: false,
    maximizable: false,
    center: true,
  });
});
