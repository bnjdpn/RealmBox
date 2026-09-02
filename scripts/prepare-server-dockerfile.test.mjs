import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { fileURLToPath } from "node:url";

test("prepares the pinned server Dockerfile without patch markers", () => {
  const directory = mkdtempSync(join(tmpdir(), "realmbox-dockerfile-"));
  const dockerfile = join(directory, "Dockerfile");
  const script = fileURLToPath(new URL("./prepare-server-dockerfile.mjs", import.meta.url));
  writeFileSync(
    dockerfile,
    [
      'ARG CTOOLS_BUILD="all"',
      'RUN cmake --build . --config "$CTYPE" -j $(($(nproc) + 1))',
      "VOLUME /azerothcore/env/dist/etc",
      "",
      'CMD ["worldserver"]',
      "",
    ].join("\n"),
  );

  try {
    execFileSync(process.execPath, [script, dockerfile]);
    const prepared = readFileSync(dockerfile, "utf8");
    assert.match(prepared, /ARG REALMBOX_BUILD_JOBS=2/);
    assert.match(prepared, /--parallel "\$REALMBOX_BUILD_JOBS"/);
    assert.match(
      prepared,
      /COPY --chown=\$DOCKER_USER:\$DOCKER_USER \\\n+     modules\/mod-playerbots\/data \/azerothcore\/modules\/mod-playerbots\/data/,
    );
    assert.doesNotMatch(prepared, /\n\+\s+modules\/mod-playerbots\/data/);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
