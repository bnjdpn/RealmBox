import { readFileSync, writeFileSync } from "node:fs";

const dockerfile = process.argv[2];
if (!dockerfile) {
  throw new Error("usage: node prepare-server-dockerfile.mjs <Dockerfile>");
}

let source = readFileSync(dockerfile, "utf8");
const replacements = [
  [
    'ARG CTOOLS_BUILD="all"',
    'ARG CTOOLS_BUILD="all"\nARG REALMBOX_BUILD_JOBS=2',
  ],
  [
    'cmake --build . --config "$CTYPE" -j $(($(nproc) + 1))',
    'cmake --build . --config "$CTYPE" --parallel "$REALMBOX_BUILD_JOBS"',
  ],
  [
    'VOLUME /azerothcore/env/dist/etc\n\nCMD ["worldserver"]',
    'COPY --chown=$DOCKER_USER:$DOCKER_USER \\\n+     modules/mod-playerbots/data /azerothcore/modules/mod-playerbots/data\n\nVOLUME /azerothcore/env/dist/etc\n\nCMD ["worldserver"]',
  ],
];

for (const [anchor, replacement] of replacements) {
  if (source.split(anchor).length !== 2) {
    throw new Error(`expected exactly one upstream anchor: ${anchor}`);
  }
  source = source.replace(anchor, replacement);
}

writeFileSync(dockerfile, source);
