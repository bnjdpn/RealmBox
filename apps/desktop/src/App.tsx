import { useEffect, useMemo, useRef, useState } from "react";
import { messages, preferredLanguage, type Copy, type Language } from "./i18n";
import {
  bootstrapLauncher,
  chooseGameData,
  getRealmDiagnostics,
  inspectAiCapability,
  inspectGameData,
  installRealm,
  startRealm,
  stopRealm,
  subscribeLauncherProgress,
  subscribeLauncherStatus,
  updatePlayerbotPopulation,
} from "./runtime";
import type { AiCapability, ClientChoice, GameDataInspection, LauncherComponent, LauncherStatus, RealmDiagnostics } from "./types";

type View = "world" | "companions" | "diagnostics";

const initialStatus: LauncherStatus = {
  phase: "checking", message: "Vérification de l’installation…", detail: null, progress: 0,
  installed: false, botsEnabled: true, botCount: 50, aiEnabled: false, aiModel: null,
  gameDataPath: null, accountName: null, accountPassword: null, clientChoice: "managedOpenWow",
  originalClientSupported: false, platformLabel: "Détection en cours", components: [],
};

const checkingAi: AiCapability = {
  state: "checking", deviceName: null, ramGb: null, modelId: null, modelName: null,
  ollamaModel: null, grade: null, estimatedTokensPerSecond: null,
  detail: "CanIRun évalue la mémoire disponible.", sourceUrl: "https://www.canirun.ai/",
};

const populationCounts = [5, 25, 50, 100, 150] as const;

function isBusy(status: LauncherStatus) {
  return ["checking", "installing", "starting", "stopping"].includes(status.phase);
}

function populationName(count: number, copy: Copy) {
  if (count <= 5) return copy.discreet;
  if (count <= 25) return copy.light;
  if (count <= 50) return copy.balanced;
  if (count <= 100) return copy.dense;
  return copy.veryDense;
}

function componentLabel(id: LauncherComponent["id"], copy: Copy) {
  return {
    client: copy.componentClient, database: copy.componentDatabase, server: copy.componentServer,
    bots: copy.componentBots, ai: copy.componentAi,
  }[id];
}

function componentDetail(component: LauncherComponent, status: LauncherStatus, copy: Copy) {
  if (component.id === "client") return status.clientChoice === "managedOpenWow" ? "OpenWoW 0.1.2" : copy.originalClient;
  if (component.id === "database") return copy.localOnly;
  if (component.id === "server") return "127.0.0.1";
  if (component.id === "bots") return status.botsEnabled ? `${status.botCount} · ${copy.team}` : copy.off;
  return status.aiEnabled ? (status.aiModel ?? copy.ai) : copy.off;
}

function stateLabel(component: LauncherComponent, copy: Copy) {
  if (component.state === "running") return copy.active;
  if (component.state === "ready") return copy.ready;
  if (component.state === "stopped") return copy.off;
  if (component.state === "missing") return copy.selected;
  return component.state === "error" ? "!" : "…";
}

function phaseCopy(status: LauncherStatus, copy: Copy) {
  if (status.phase === "running") return { title: copy.runningTitle, body: copy.runningBody };
  if (status.phase === "ready") return { title: copy.readyTitle, body: copy.readyBody };
  if (status.phase === "installing") return { title: copy.installingTitle, body: localizedOperation(status.message, copy) };
  if (status.phase === "starting") return { title: copy.startingTitle, body: localizedOperation(status.message, copy) };
  if (status.phase === "stopping") return { title: copy.stoppingTitle, body: localizedOperation(status.message, copy) };
  if (status.phase === "checking") return { title: copy.checkingTitle, body: copy.checkingBody };
  if (status.phase === "error") return { title: copy.errorTitle, body: copy.genericError };
  return { title: copy.installTitle, body: copy.installBody };
}

function localizedOperation(message: string, copy: Copy) {
  if (copy.localOnly === "LOCAL UNIQUEMENT") return message;
  const translations: Record<string, string> = {
    "Validation des données 3.3.5a": "Validating 3.3.5a game data",
    "Téléchargement du client OpenWoW": "Downloading the OpenWoW client",
    "Vérification du client fourni": "Checking your game client",
    "Préparation du serveur précompilé": "Preparing the prebuilt server",
    "Téléchargement du serveur épinglé": "Downloading the pinned server",
    "Installation du module Playerbots": "Installing Playerbots",
    "Ajout des dialogues locaux": "Adding local dialogue",
    "Téléchargement du moteur de dialogue": "Downloading the local dialogue engine",
    "Téléchargement du serveur local": "Downloading the local server",
    "Construction du serveur local": "Building the local server",
    "Préparation de la base locale": "Preparing the local save",
    "Création du compte local": "Creating the local account",
    "Préparation du modèle local": "Preparing the local model",
    "Finalisation de RealmBox": "Finishing RealmBox setup",
    "Démarrage de la base locale": "Starting the local save",
    "Vérification du monde": "Checking the world",
    "Réveil des dialogues locaux": "Starting local dialogue",
    "Réveil du serveur et des compagnons": "Starting the server and companions",
    "Réveil du serveur": "Starting the server",
    "Ouverture du client": "Opening the game client",
    "Fermeture du client": "Closing the game client",
    "Arrêt du serveur local": "Stopping the local server",
  };
  return translations[message] ?? copy.genericError;
}

function errorCopy(detail: string | null, copy: Copy) {
  const value = (detail ?? "").toLowerCase();
  if (value.includes("docker")) return { cause: copy.dockerError, recovery: copy.dockerRecovery };
  if (value.includes("data") || value.includes("mpq") || value.includes("archive")) return { cause: copy.dataError, recovery: copy.dataRecovery };
  if (value.includes("curl") || value.includes("télécharg") || value.includes("download")) return { cause: copy.downloadError, recovery: copy.downloadRecovery };
  if (value.includes("client") || value.includes("openwow") || value.includes("wow.exe")) return { cause: copy.clientError, recovery: copy.clientRecovery };
  if (value.includes("server") || value.includes("serveur") || value.includes("port")) return { cause: copy.serverError, recovery: copy.serverRecovery };
  return { cause: copy.genericError, recovery: copy.genericRecovery };
}

export default function App() {
  const [language, setLanguage] = useState<Language>(() => preferredLanguage());
  const copy = messages[language];
  const [view, setView] = useState<View>("world");
  const [status, setStatus] = useState(initialStatus);
  const [gameDataPath, setGameDataPath] = useState<string | null>(null);
  const [gameDataInspection, setGameDataInspection] = useState<GameDataInspection | null>(null);
  const [gameDataError, setGameDataError] = useState<string | null>(null);
  const [botsEnabled, setBotsEnabled] = useState(true);
  const [botCount, setBotCount] = useState(50);
  const [clientChoice, setClientChoice] = useState<ClientChoice>("managedOpenWow");
  const [aiEnabled, setAiEnabled] = useState(false);
  const [aiCapability, setAiCapability] = useState<AiCapability>(checkingAi);
  const [requestPending, setRequestPending] = useState(false);
  const [populationFeedback, setPopulationFeedback] = useState<string | null>(null);
  const [diagnostics, setDiagnostics] = useState<RealmDiagnostics | null>(null);
  const [diagnosticPending, setDiagnosticPending] = useState(false);
  const [copied, setCopied] = useState(false);
  const bootstrapRequest = useRef<Promise<LauncherStatus> | null>(null);
  const aiRequest = useRef<Promise<AiCapability> | null>(null);
  const aiChoiceTouched = useRef(false);
  const installationKnown = useRef(false);

  useEffect(() => {
    let active = true;
    let unlistenProgress: () => void = () => undefined;
    let unlistenStatus: () => void = () => undefined;
    void subscribeLauncherProgress((progress) => active && setStatus((current) => ({ ...current, ...progress })))
      .then((unlisten) => { unlistenProgress = unlisten; });
    void subscribeLauncherStatus((next) => {
      if (!active) return;
      setStatus(next); setBotsEnabled(next.botsEnabled); setBotCount(next.botCount);
      setAiEnabled(next.aiEnabled); setClientChoice(next.clientChoice);
    }).then((unlisten) => { unlistenStatus = unlisten; });
    bootstrapRequest.current ??= bootstrapLauncher();
    void bootstrapRequest.current.then((next) => {
      if (!active) return;
      setStatus(next); installationKnown.current = next.installed; setGameDataPath(next.gameDataPath);
      setBotsEnabled(next.botsEnabled); setBotCount(next.botCount); setAiEnabled(next.aiEnabled); setClientChoice(next.clientChoice);
      if (next.aiModel) setAiCapability({ ...checkingAi, state: "recommended", modelName: next.aiModel, ollamaModel: next.aiModel });
    }).catch((error: unknown) => active && setStatus({ ...initialStatus, phase: "error", detail: String(error) }));
    aiRequest.current ??= inspectAiCapability();
    void aiRequest.current.then((capability) => {
      if (!active) return;
      setAiCapability(capability);
      if (capability.state === "recommended" && !aiChoiceTouched.current && !installationKnown.current) setAiEnabled(true);
    });
    return () => { active = false; unlistenProgress(); unlistenStatus(); };
  }, []);

  useEffect(() => {
    localStorage.setItem("realmbox-language", language);
    document.documentElement.lang = language;
  }, [language]);

  const phase = useMemo(() => phaseCopy(status, copy), [status, copy]);
  const presentedError = errorCopy(status.detail, copy);
  const displayedPlatform = language === "en" && status.platformLabel === "Aperçu navigateur" ? "Browser preview" : status.platformLabel;

  async function selectData() {
    setRequestPending(true); setGameDataError(null);
    try {
      const selected = await chooseGameData();
      if (!selected) return;
      const inspection = await inspectGameData(selected);
      setGameDataInspection(inspection); setGameDataPath(inspection.path);
    } catch (error) {
      setGameDataInspection(null); setGameDataPath(null); setGameDataError(String(error));
    } finally { setRequestPending(false); }
  }

  async function install() {
    if (!gameDataPath) return;
    setRequestPending(true);
    try { setStatus(await installRealm(gameDataPath, clientChoice, botsEnabled, botCount, aiEnabled, aiCapability.ollamaModel)); }
    catch (error) { setStatus((current) => ({ ...current, phase: "error", detail: String(error) })); }
    finally { setRequestPending(false); }
  }

  async function start() {
    setRequestPending(true);
    try { setStatus(await startRealm(botsEnabled, botCount, aiEnabled)); }
    catch (error) { setStatus((current) => ({ ...current, phase: "error", detail: String(error) })); }
    finally { setRequestPending(false); }
  }

  async function stop() {
    setRequestPending(true);
    try { setStatus(await stopRealm()); }
    catch (error) { setStatus((current) => ({ ...current, phase: "error", detail: String(error) })); }
    finally { setRequestPending(false); }
  }

  async function applyPopulation() {
    setRequestPending(true); setPopulationFeedback(null);
    try {
      const next = await updatePlayerbotPopulation(botsEnabled, botCount);
      setStatus(next); setBotCount(next.botCount); setPopulationFeedback(copy.applied);
    } catch (error) {
      setStatus((current) => ({ ...current, phase: "error", detail: String(error) })); setView("world");
    } finally { setRequestPending(false); }
  }

  async function refreshDiagnostics() {
    setDiagnosticPending(true);
    try { setDiagnostics(await getRealmDiagnostics()); }
    catch { setDiagnostics(null); }
    finally { setDiagnosticPending(false); }
  }

  function selectView(next: View) {
    setView(next);
    if (next === "diagnostics") void refreshDiagnostics();
  }

  async function copyDiagnostics() {
    if (!diagnostics || !navigator.clipboard) return;
    const payload = [diagnostics.summary, `component=${diagnostics.component}`, `logs=${diagnostics.logsPath}`, ...diagnostics.recentEntries].join("\n");
    await navigator.clipboard.writeText(payload); setCopied(true);
    window.setTimeout(() => setCopied(false), 1800);
  }

  const populationSelect = (
    <select value={botCount} onChange={(event) => { setBotCount(Number(event.target.value)); setPopulationFeedback(null); }}>
      {populationCounts.map((count) => <option key={count} value={count}>{populationName(count, copy)} · {count}</option>)}
    </select>
  );

  return (
    <main className="app-shell">
      <section className="app-window">
        <header className="topbar">
          <div className="brand"><span className="brand-glyph" aria-hidden="true">R</span><strong>RealmBox</strong><small>3.3.5a</small></div>
          <div className="local-status"><span className={`status-dot ${status.phase}`} />{copy.localOnly}</div>
          <div className="language-switch" aria-label={copy.language}>
            <button className={language === "fr" ? "active" : ""} onClick={() => setLanguage("fr")}>FR</button>
            <button className={language === "en" ? "active" : ""} onClick={() => setLanguage("en")}>EN</button>
          </div>
        </header>

        <div className="workspace">
          <aside className="sidebar">
            <nav aria-label="RealmBox">
              <button className={view === "world" ? "active" : ""} onClick={() => selectView("world")}><span>◆</span>{copy.world}</button>
              <button className={view === "companions" ? "active" : ""} onClick={() => selectView("companions")} disabled={!status.installed}><span>♟</span>{copy.companions}</button>
              <button className={view === "diagnostics" ? "active" : ""} onClick={() => selectView("diagnostics")}><span>⋯</span>{copy.diagnostics}</button>
            </nav>
            <div className="sidebar-art" aria-hidden="true" />
            <p>{displayedPlatform}</p>
          </aside>

          <section className="content" aria-live="polite">
            {view === "world" && <>
              <header className="page-heading">
                <div><span className="eyebrow">{status.phase === "running" ? copy.active : status.phase === "ready" ? copy.ready : copy.progress}</span><h1>{phase.title}</h1><p>{phase.body}</p></div>
                <div className={`phase-orb ${status.phase}`} aria-hidden="true" />
              </header>

              {status.phase === "needsGameData" && <section className="card setup-flow">
                <fieldset><legend>1 · {copy.gameClient}</legend>
                  <label className="choice"><input type="radio" name="client" checked={clientChoice === "managedOpenWow"} onChange={() => setClientChoice("managedOpenWow")} /><span><strong>{copy.managedClient}</strong><small>{copy.managedClientHelp}</small></span></label>
                  <label className={`choice ${!status.originalClientSupported ? "muted" : ""}`}><input type="radio" name="client" checked={clientChoice === "originalWindows"} disabled={!status.originalClientSupported} onChange={() => setClientChoice("originalWindows")} /><span><strong>{copy.originalClient}</strong><small>{status.originalClientSupported ? copy.originalClientHelp : copy.originalUnavailable}</small></span></label>
                </fieldset>
                <fieldset><legend>2 · {copy.gameData}</legend>
                  <button className="folder-button" onClick={selectData} disabled={requestPending}><span>{gameDataPath ?? copy.chooseData}</span><b>{copy.browse}</b></button>
                  <p className={`helper ${gameDataError ? "error" : gameDataInspection ? "success" : ""}`}>{gameDataError ?? (gameDataInspection ? `Data ${gameDataInspection.locale} · build 12340` : copy.dataRequirement)}</p>
                </fieldset>
                <fieldset><legend>3 · {copy.companions}</legend>
                  <label className="choice"><input type="checkbox" checked={botsEnabled} onChange={(event) => { setBotsEnabled(event.target.checked); if (!event.target.checked) setAiEnabled(false); }} /><span><strong>{copy.populate}</strong><small>{copy.populateHelp}</small></span></label>
                  {botsEnabled && <label className="inline-field"><span><strong>{copy.population}</strong><small>{copy.populationHelp}</small></span>{populationSelect}</label>}
                  <label className={`choice ${aiCapability.state !== "recommended" ? "muted" : ""}`}><input type="checkbox" checked={aiEnabled} disabled={!botsEnabled || aiCapability.state !== "recommended"} onChange={(event) => { aiChoiceTouched.current = true; setAiEnabled(event.target.checked); }} /><span><strong>{copy.ai}</strong><small>{aiCapability.state === "checking" ? copy.aiChecking : aiCapability.state === "recommended" ? `${aiCapability.modelName} · ${aiCapability.grade ?? ""} · ~${aiCapability.estimatedTokensPerSecond ?? "?"} tok/s` : copy.aiUnavailable}</small></span></label>
                  <p className="helper">{copy.aiPrivacy}</p>
                </fieldset>
              </section>}

              {status.phase === "error" && <section className="recovery-card"><div><span>{copy.cause}</span><strong>{presentedError.cause}</strong></div><div><span>{copy.recovery}</span><strong>{presentedError.recovery}</strong></div><button onClick={() => void bootstrapLauncher().then(setStatus)}>{copy.checkAgain}</button></section>}

              {status.components.length > 0 && status.phase !== "needsGameData" && <section className="component-grid">
                {status.components.map((component) => <article key={component.id}><span className={`component-icon ${component.state}`} aria-hidden="true" /><div><strong>{componentLabel(component.id, copy)}</strong><small>{componentDetail(component, status, copy)}</small></div><em>{stateLabel(component, copy)}</em></article>)}
              </section>}

              {status.installed && status.accountName && <section className="account-strip"><div><span>{copy.account}</span><strong>{status.accountName} / {status.accountPassword}</strong></div><small>{copy.accountHelp}</small></section>}
            </>}

            {view === "companions" && <>
              <header className="page-heading"><div><span className="eyebrow">{copy.companions}</span><h1>{copy.companionsTitle}</h1><p>{status.phase === "running" ? copy.companionsBodyRunning : copy.companionsBodyReady}</p></div></header>
              <section className="card population-card">
                <label className="choice"><input type="checkbox" checked={botsEnabled} onChange={(event) => setBotsEnabled(event.target.checked)} /><span><strong>{copy.populate}</strong><small>{copy.populateHelp}</small></span></label>
                {botsEnabled && <label className="inline-field large"><span><strong>{copy.requestedPopulation}</strong><small>{copy.populationHelp}</small></span>{populationSelect}</label>}
                <div className="team-note"><span aria-hidden="true">♟</span><div><strong>{copy.team}</strong><p>{copy.teamHelp}</p></div></div>
                {status.phase === "running" ? <button className="secondary-action" onClick={applyPopulation} disabled={requestPending}>{copy.applyNow}</button> : <p className="helper">{copy.startToApply}</p>}
                {populationFeedback && <p className="success-message">✓ {populationFeedback}</p>}
              </section>
            </>}

            {view === "diagnostics" && <>
              <header className="page-heading"><div><span className="eyebrow">{copy.localOnly}</span><h1>{copy.diagnosticsTitle}</h1><p>{copy.diagnosticsBody}</p></div><button className="quiet-button" onClick={refreshDiagnostics} disabled={diagnosticPending}>{copy.refresh}</button></header>
              <section className="card diagnostic-card">
                {diagnostics ? <>
                  <dl><div><dt>{copy.affectedComponent}</dt><dd>{({ client: copy.componentClient, database: copy.componentDatabase, server: copy.componentServer, bots: copy.componentBots, ai: copy.componentAi, launcher: copy.componentLauncher })[diagnostics.component]}</dd></div><div><dt>{copy.logsFolder}</dt><dd>{diagnostics.logsPath}</dd></div></dl>
                  <p className="diagnostic-summary">{diagnostics.summary}</p>
                  <div className="log-list">{diagnostics.recentEntries.length ? diagnostics.recentEntries.map((entry, index) => <code key={`${index}-${entry}`}>{entry}</code>) : <span>{copy.noRecentErrors}</span>}</div>
                  {status.phase === "error" && status.detail && <details><summary>{copy.cause}</summary><code>{status.detail}</code></details>}
                  <button className="secondary-action" onClick={copyDiagnostics}>{copied ? copy.copied : copy.copy}</button>
                </> : <p>{copy.noDiagnostic}</p>}
              </section>
            </>}
          </section>
        </div>

        <footer className="actionbar">
          <div className="progress-block"><div><span style={{ width: `${status.progress}%` }} /></div><small>{copy.progress} · {status.progress}%</small></div>
          {view === "world" && status.phase === "needsGameData" && <button className="primary-action" onClick={install} disabled={!gameDataPath || requestPending}>{copy.install}</button>}
          {view === "world" && status.phase === "ready" && <button className="primary-action" onClick={start} disabled={requestPending}>{copy.play}</button>}
          {view === "world" && status.phase === "running" && <button className="primary-action stop" onClick={stop} disabled={requestPending}>{copy.stop}</button>}
          {view === "world" && isBusy(status) && <button className="primary-action" disabled>{copy.wait}</button>}
        </footer>
      </section>
      <p className="version-line">RealmBox 0.2.0 · {displayedPlatform} · {copy.versionSuffix}</p>
    </main>
  );
}
