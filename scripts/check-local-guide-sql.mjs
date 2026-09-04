// Opt-in integration proof. Never mounts RealmBox's runtime or player volumes.
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { readFileSync } from "node:fs";
import { setTimeout as delay } from "node:timers/promises";

const source = readFileSync(new URL("../apps/desktop/src-tauri/src/launcher.rs", import.meta.url), "utf8");
const image = source.match(/const MYSQL_IMAGE: &str\s*=\s*"([^"]+)"/)[1];
const questSql = source.match(/const LOCAL_GUIDE_QUEST_SQL: &str = r#"([^]*?)"#;/)[1];
const itemSql = source.match(/const LOCAL_GUIDE_ITEM_SQL: &str = r#"([^]*?)"#;/)[1];
const docker = (args, input) => execFileSync("docker", args, {
  encoding: "utf8", input, timeout: 30_000, stdio: ["pipe", "pipe", "pipe"],
}).trim();

const container = docker([
  "run", "--detach", "--rm", "--network", "none",
  "--label", "org.realmbox.test=local-guide-sql",
  "--tmpfs", "/var/lib/mysql:rw,size=536870912",
  "--memory", "768m", "--cpus", "1", "--stop-timeout", "5",
  "--env", "MYSQL_ALLOW_EMPTY_PASSWORD=yes", image,
]);
assert.match(container, /^[a-f0-9]{64}$/);

try {
  let ready = false;
  for (let attempt = 0; attempt < 120; attempt += 1) {
    try {
      docker(["exec", container, "mysqladmin", "ping", "--silent"]);
      ready = true;
      break;
    } catch { await delay(500); }
  }
  assert.ok(ready, "isolated MySQL did not become ready");
  docker(["exec", "-i", container, "mysql", "--user=root"], `
    SET NAMES utf8mb4;
    CREATE DATABASE acore_world CHARACTER SET utf8mb4;
    USE acore_world;
    CREATE TABLE quest_template (ID INT, QuestLevel INT, LogTitle VARCHAR(255), QuestDescription TEXT, LogDescription TEXT);
    CREATE TABLE quest_template_locale (ID INT, locale VARCHAR(4), Title VARCHAR(255), Details TEXT);
    CREATE TABLE item_template (entry INT, name VARCHAR(255), description TEXT, RequiredLevel INT, ItemLevel INT);
    CREATE TABLE item_template_locale (ID INT, locale VARCHAR(4), Name VARCHAR(255), Description TEXT);
    INSERT INTO quest_template VALUES (17, 5, 'Test quest', 'Synthetic reference.', 'Synthetic objective.');
    INSERT INTO quest_template_locale VALUES (17, 'frFR', 'Épreuve locale', REPEAT('é', 500));
    INSERT INTO item_template VALUES (23, 'Test sword', 'Synthetic item.', 3, 8);
    INSERT INTO item_template_locale VALUES (23, 'frFR', 'Épée de test', 'Objet fictif.');
  `);
  const query = (sql, term, locale) => docker([
    "exec", "--env", `REALMBOX_GUIDE_TERM_HEX=${Buffer.from(term).toString("hex")}`,
    container, "sh", "-c",
    `exec mysql --batch --raw --skip-column-names --connect-timeout=5 --user=root --database=acore_world --execute="${sql.replace("__LOCALE__", locale)}"`,
  ]).split("\t");
  const french = query(questSql, "épreuve", "frFR");
  assert.equal(french.length, 5, "trimmed output must preserve all TSV columns");
  assert.equal(Buffer.from(french[1], "hex").toString(), "Épreuve locale");
  assert.equal(Array.from(Buffer.from(french[2], "hex").toString()).length, 321);
  const english = query(questSql, "test", "enUS");
  assert.equal(Buffer.from(english[1], "hex").toString(), "Test quest");
  const item = query(itemSql, "épée", "frFR");
  assert.equal(item.length, 5);
  assert.equal(Buffer.from(item[1], "hex").toString(), "Épée de test");
  assert.equal(item[3], "3");
  assert.throws(() => docker([
    "exec", container, "mysql", "--user=root", "--database=acore_world",
    "--execute=START TRANSACTION READ ONLY; INSERT INTO quest_template (ID) VALUES (99); COMMIT;",
  ]), /read only|read-only/i, "MySQL must reject writes in the guide transaction");
  assert.equal(docker([
    "exec", container, "mysql", "--user=root", "--batch", "--skip-column-names",
    "--database=acore_world", "--execute=SELECT COUNT(*) FROM quest_template WHERE ID = 99;",
  ]), "0");
  console.log("Local guide SQL: French/English lookup, bounded fields, 5-column framing and read-only rejection passed on isolated MySQL.");
} finally {
  docker(["stop", container]);
}
