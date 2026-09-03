import { useEffect, useMemo, useRef, useState } from "react";
import { messages, preferredLanguage, type Copy, type Language } from "./i18n";
import realmIcon from "./assets/realmbox-icon.svg";
import {
  bootstrapLauncher,
  changeGameDataPath,
  chooseGameData,
  configureLocalDialogue,
  configureDialogueChattiness,
  getRealmDiagnostics,
  inspectAiCapability,
  inspectGameData,
  installRealm,
  restoreLastRecovery,
  startRealm,
  stopRealm,
  subscribeLauncherProgress,
  subscribeLauncherStatus,
  updatePlayerbotPopulation,
} from "./runtime";
import type { AiCapability, ClientChoice, DialogueChattiness, GameDataInspection, LauncherCommandError, LauncherErrorCode, LauncherStatus, RealmDiagnostics } from "./types";

type Panel = "settings" | "companions" | "dialogues" | "diagnostics";
type WorldProfile = "quiet" | "balanced" | "dense" | "custom";

const initialStatus: LauncherStatus = {
  phase: "checking", message: "Vérification de l’installation…", detail: null, errorCode: null, progress: 0,
  installed: false, recoveryAvailable: false, botsEnabled: true, botCount: 50, requestedBotCount: 50, appliedBotCount: 50, aiEnabled: false, aiModel: null,
  dialogueChattiness: "balanced",
  gameDataPath: null, accountName: null, accountPassword: null, clientChoice: "managedOpenWow",
  originalClientSupported: false, platformLabel: "Détection en cours", components: [],
};

const checkingAi: AiCapability = {
  state: "checking", deviceName: null, ramGb: null, modelId: null, modelName: null,
  ollamaModel: null, grade: null, estimatedTokensPerSecond: null,
  downloadSizeGb: null,
  diskAvailableGb: null,
  diskSpaceSufficient: null,
  modelLicense: null,
  detail: "CanIRun évalue la mémoire disponible.", sourceUrl: "https://www.canirun.ai/",
};

const populationCounts = [5, 25, 50, 100, 150] as const;

function isBusy(status: LauncherStatus) {
  return ["checking", "installing", "starting", "stopping", "recovering"].includes(status.phase);
}

function populationName(count: number, copy: Copy) {
  if (count <= 5) return copy.discreet;
  if (count <= 25) return copy.light;
  if (count <= 50) return copy.balanced;
  if (count <= 100) return copy.dense;
  return copy.veryDense;
}

function profileForPopulation(count: number): WorldProfile {
  if (count === 25) return "quiet";
  if (count === 50) return "balanced";
  if (count === 100) return "dense";
  return "custom";
}

function formatBytes(bytes: number, language: Language) {
  const units = ["o", "Ko", "Mo", "Go"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  return `${new Intl.NumberFormat(language, { maximumFractionDigits: unit === 0 ? 0 : 1 }).format(value)} ${units[unit]}`;
}

function phaseCopy(status: LauncherStatus, copy: Copy) {
  if (status.phase === "running") return { title: copy.runningTitle, body: copy.runningBody };
  if (status.phase === "ready") return { title: copy.readyTitle, body: copy.readyBody };
  if (status.phase === "installing") return { title: copy.installingTitle, body: localizedOperation(status.message, copy) };
  if (status.phase === "starting") return { title: copy.startingTitle, body: localizedOperation(status.message, copy) };
  if (status.phase === "stopping") return { title: copy.stoppingTitle, body: localizedOperation(status.message, copy) };
  if (status.phase === "recovering") return { title: copy.recoveringTitle, body: localizedOperation(status.message, copy) };
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
    "Vérification du point de restauration": "Checking the recovery point",
    "Conservation de l’état actuel": "Preserving the current state",
    "Restauration des personnages": "Restoring characters",
    "Dernier état fonctionnel restauré": "Last working state restored",
  };
  return translations[message] ?? copy.genericError;
}

function errorCopy(code: LauncherErrorCode | null, copy: Copy) {
  if (code === "dockerMissing" || code === "dockerNotRunning") return { cause: copy.dockerError, recovery: copy.dockerRecovery };
  if (code === "gameDataIncomplete" || code === "gameBuildUnsupported") return { cause: copy.dataError, recovery: copy.dataRecovery };
  if (code === "downloadInterrupted" || code === "checksumMismatch") return { cause: copy.downloadError, recovery: copy.downloadRecovery };
  if (code === "clientLaunchFailed") return { cause: copy.clientError, recovery: copy.clientRecovery };
  if (["portUnavailable", "backupFailed", "migrationFailed", "recoveryFailed", "worldServerTimeout", "installationIncomplete"].includes(code ?? "")) return { cause: copy.serverError, recovery: copy.serverRecovery };
  return { cause: copy.genericError, recovery: copy.genericRecovery };
}

function commandError(error: unknown): Pick<LauncherStatus, "detail" | "errorCode"> {
  if (error && typeof error === "object" && "code" in error) {
    const typed = error as Partial<LauncherCommandError>;
    return {
      detail: typeof typed.technicalDetail === "string" ? typed.technicalDetail : null,
      errorCode: typeof typed.code === "string" ? typed.code : "unknown",
    };
  }
  return { detail: String(error), errorCode: "unknown" };
}

export default function App() {
  const [language, setLanguage] = useState<Language>(() => preferredLanguage());
  const copy = messages[language];
  const [panel, setPanel] = useState<Panel | null>(null);
  const panelRef = useRef<HTMLElement | null>(null);
  const panelTriggerRef = useRef<HTMLElement | null>(null);
  const restorePanelFocusRef = useRef(false);
  const [status, setStatus] = useState(initialStatus);
  const [gameDataPath, setGameDataPath] = useState<string | null>(null);
  const [gameDataInspection, setGameDataInspection] = useState<GameDataInspection | null>(null);
  const [gameDataError, setGameDataError] = useState<string | null>(null);
  const [gameDataFeedback, setGameDataFeedback] = useState<string | null>(null);
  const [botsEnabled, setBotsEnabled] = useState(true);
  const [botCount, setBotCount] = useState(50);
  const [worldProfile, setWorldProfile] = useState<WorldProfile>("balanced");
  const [clientChoice, setClientChoice] = useState<ClientChoice>("managedOpenWow");
  const [aiEnabled, setAiEnabled] = useState(false);
  const [aiCapability, setAiCapability] = useState<AiCapability>(checkingAi);
  const [requestPending, setRequestPending] = useState(false);
  const [populationFeedback, setPopulationFeedback] = useState<string | null>(null);
  const [dialogueFeedback, setDialogueFeedback] = useState<string | null>(null);
  const [dialogueError, setDialogueError] = useState<string | null>(null);
  const [diagnostics, setDiagnostics] = useState<RealmDiagnostics | null>(null);
  const [diagnosticPending, setDiagnosticPending] = useState(false);
  const [copied, setCopied] = useState(false);
  const bootstrapRequest = useRef<Promise<LauncherStatus> | null>(null);
  const aiRequest = useRef<Promise<AiCapability> | null>(null);

  useEffect(() => {
    let active = true;
    let unlistenProgress: () => void = () => undefined;
    let unlistenStatus: () => void = () => undefined;
    void subscribeLauncherProgress((progress) => active && setStatus((current) => ({ ...current, ...progress })))
      .then((unlisten) => { unlistenProgress = unlisten; });
    void subscribeLauncherStatus((next) => {
      if (!active) return;
      setStatus(next); setBotsEnabled(next.botsEnabled); setBotCount(next.requestedBotCount); setWorldProfile(profileForPopulation(next.requestedBotCount));
      setAiEnabled(next.aiEnabled); setClientChoice(next.clientChoice); setGameDataPath(next.gameDataPath);
    }).then((unlisten) => { unlistenStatus = unlisten; });
    bootstrapRequest.current ??= bootstrapLauncher();
    void bootstrapRequest.current.then((next) => {
      if (!active) return;
      setStatus(next); setGameDataPath(next.gameDataPath);
      setBotsEnabled(next.botsEnabled); setBotCount(next.requestedBotCount); setWorldProfile(profileForPopulation(next.requestedBotCount)); setAiEnabled(next.aiEnabled); setClientChoice(next.clientChoice);
      if (next.aiModel) setAiCapability({ ...checkingAi, state: "recommended", modelName: next.aiModel, ollamaModel: next.aiModel });
    }).catch((error: unknown) => active && setStatus({ ...initialStatus, phase: "error", ...commandError(error) }));
    aiRequest.current ??= inspectAiCapability();
    void aiRequest.current.then((capability) => {
      if (!active) return;
      setAiCapability(capability);
    });
    return () => { active = false; unlistenProgress(); unlistenStatus(); };
  }, []);

  useEffect(() => {
    localStorage.setItem("realmbox-language", language);
    document.documentElement.lang = language;
  }, [language]);

  useEffect(() => {
    if (!panel) {
      if (restorePanelFocusRef.current) {
        panelTriggerRef.current?.focus();
        restorePanelFocusRef.current = false;
      }
      return;
    }
    const dialog = panelRef.current;
    const focusable = () => Array.from(dialog?.querySelectorAll<HTMLElement>("button:not([disabled]), input:not([disabled]), select:not([disabled]), textarea:not([disabled]), a[href], [tabindex]:not([tabindex='-1'])") ?? []);
    focusable()[0]?.focus();
    const keepFocusInside = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        restorePanelFocusRef.current = true;
        setPanel(null);
        return;
      }
      if (event.key !== "Tab") return;
      const elements = focusable();
      if (!elements.length) return;
      const first = elements[0];
      const last = elements[elements.length - 1];
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault(); last.focus();
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault(); first.focus();
      }
    };
    dialog?.addEventListener("keydown", keepFocusInside);
    return () => dialog?.removeEventListener("keydown", keepFocusInside);
  }, [panel]);

  const phase = useMemo(() => phaseCopy(status, copy), [status, copy]);
  const presentedError = errorCopy(status.errorCode, copy);
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
    try { setStatus(await installRealm(gameDataPath, clientChoice, botsEnabled, botCount, aiEnabled, aiEnabled ? aiCapability.ollamaModel : null)); }
    catch (error) { setStatus((current) => ({ ...current, phase: "error", ...commandError(error) })); }
    finally { setRequestPending(false); }
  }

  async function changeInstalledGameData() {
    setRequestPending(true); setGameDataError(null); setGameDataFeedback(null);
    try {
      const selected = await chooseGameData();
      if (!selected) return;
      const inspection = await inspectGameData(selected);
      const next = await changeGameDataPath(inspection.path);
      setStatus(next); setGameDataPath(next.gameDataPath); setGameDataInspection(inspection);
      setGameDataFeedback(copy.gameFolderUpdated);
    } catch (error) {
      setGameDataFeedback(null); setGameDataError(String(error));
    } finally { setRequestPending(false); }
  }

  async function start() {
    setRequestPending(true);
    try { setStatus(await startRealm(botsEnabled, botCount, aiEnabled)); }
    catch (error) { setStatus((current) => ({ ...current, phase: "error", ...commandError(error) })); }
    finally { setRequestPending(false); }
  }

  async function stop() {
    setRequestPending(true);
    try { setStatus(await stopRealm()); }
    catch (error) { setStatus((current) => ({ ...current, phase: "error", ...commandError(error) })); }
    finally { setRequestPending(false); }
  }

  async function retry() {
    setRequestPending(true);
    setStatus((current) => ({
      ...current,
      phase: "checking",
      message: copy.checkingBody,
      detail: null,
      errorCode: null,
      progress: 0,
    }));
    try { setStatus(await bootstrapLauncher()); }
    catch (error) { setStatus((current) => ({ ...current, phase: "error", ...commandError(error) })); }
    finally { setRequestPending(false); }
  }

  async function restoreRecovery() {
    setRequestPending(true);
    try { setStatus(await restoreLastRecovery()); }
    catch (error) { setStatus((current) => ({ ...current, phase: "error", ...commandError(error) })); }
    finally { setRequestPending(false); }
  }

  async function applyPopulation() {
    setRequestPending(true); setPopulationFeedback(null);
    try {
      const next = await updatePlayerbotPopulation(botsEnabled, botCount);
      setStatus(next); setBotCount(next.requestedBotCount); setPopulationFeedback(copy.applied);
    } catch (error) {
      setStatus((current) => ({ ...current, phase: "error", ...commandError(error) })); setPanel(null);
    } finally { setRequestPending(false); }
  }

  async function configureDialogue(enabled: boolean) {
    if (enabled && !aiCapability.ollamaModel) return;
    const previousPhase = status.phase;
    setRequestPending(true); setDialogueFeedback(null); setDialogueError(null);
    try {
      const next = await configureLocalDialogue(enabled, enabled ? aiCapability.ollamaModel : status.aiModel);
      setStatus(next); setAiEnabled(next.aiEnabled);
      setDialogueFeedback(enabled ? copy.dialogueReady : copy.off);
    } catch (error) {
      setDialogueError(`${copy.dialogueFailed} ${String(error)}`);
      setStatus((current) => ({ ...current, phase: previousPhase, progress: previousPhase === "ready" || previousPhase === "running" ? 100 : current.progress }));
    } finally { setRequestPending(false); }
  }

  async function changeChattiness(chattiness: DialogueChattiness) {
    setRequestPending(true); setDialogueFeedback(null); setDialogueError(null);
    try {
      const next = await configureDialogueChattiness(chattiness);
      setStatus(next); setDialogueFeedback(copy.chattinessSaved);
    } catch (error) {
      setDialogueError(String(error));
    } finally { setRequestPending(false); }
  }

  async function refreshDiagnostics() {
    setDiagnosticPending(true);
    try { setDiagnostics(await getRealmDiagnostics()); }
    catch { setDiagnostics(null); }
    finally { setDiagnosticPending(false); }
  }

  function openPanel(next: Panel) {
    if (!panel && document.activeElement instanceof HTMLElement) panelTriggerRef.current = document.activeElement;
    setPanel(next);
    if (next === "diagnostics") void refreshDiagnostics();
  }

  function closePanel() {
    restorePanelFocusRef.current = true;
    setPanel(null);
  }

  async function copyDiagnostics() {
    if (!diagnostics || !navigator.clipboard) return;
    const payload = [diagnostics.summary, `component=${diagnostics.component}`, "logs=[local path omitted]", ...diagnostics.recentEntries].join("\n");
    await navigator.clipboard.writeText(payload); setCopied(true);
    window.setTimeout(() => setCopied(false), 1800);
  }

  const populationSelect = (
    <select value={botCount} onChange={(event) => { const count = Number(event.target.value); setBotCount(count); setWorldProfile(profileForPopulation(count)); setPopulationFeedback(null); }}>
      {populationCounts.map((count) => <option key={count} value={count}>{populationName(count, copy)} · {count}</option>)}
    </select>
  );
  const profileSelect = (
    <select value={worldProfile} onChange={(event) => {
      const profile = event.target.value as WorldProfile;
      setWorldProfile(profile);
      if (profile === "quiet") setBotCount(25);
      if (profile === "balanced") setBotCount(50);
      if (profile === "dense") setBotCount(100);
      setPopulationFeedback(null);
    }}>
      <option value="quiet">{copy.quietProfile}</option>
      <option value="balanced">{copy.balancedProfile}</option>
      <option value="dense">{copy.denseProfile}</option>
      <option value="custom">{copy.customProfile}</option>
    </select>
  );

  const panelTitle = panel === "companions" ? copy.companionsTitle : panel === "dialogues" ? copy.dialoguesTitle : panel === "diagnostics" ? copy.diagnosticsTitle : copy.settings;
  const showProgress = ["installing", "starting", "stopping", "recovering"].includes(status.phase) && status.progress > 0 && status.progress < 100;
  const progressVolume = status.completedBytes != null
    ? status.totalBytes != null
      ? `${formatBytes(status.completedBytes, language)} / ${formatBytes(status.totalBytes, language)}`
      : formatBytes(status.completedBytes, language)
    : null;

  return (
    <main className="app-shell">
      <section className={`app-window phase-${status.phase}`} aria-busy={isBusy(status)}>
        <div className="scene-shade" aria-hidden="true" />

        <div className="launcher-brand" aria-label="RealmBox">
          <img src={realmIcon} alt="" />
          <span>RealmBox</span>
        </div>

        <button className="settings-button" onClick={() => openPanel("settings")}>{copy.settings}</button>

        <section className="status-card" aria-live="polite">
          <div className={`status-marker ${status.phase}`} aria-hidden="true" />
          <h1>{status.phase === "error" ? presentedError.cause : phase.title}</h1>
          <p>{status.phase === "error" ? presentedError.recovery : phase.body}</p>

          {status.phase === "needsGameData" && gameDataPath && <div className="selected-data">
            <strong>{gameDataInspection ? `Data ${gameDataInspection.locale} · build 12340` : copy.dataReady}</strong>
            <span>{gameDataPath}</span>
          </div>}
          {status.phase === "needsGameData" && gameDataError && <p className="inline-error">{gameDataError}</p>}

          {showProgress && <div className="active-progress" role="progressbar" aria-label={copy.progress} aria-valuemin={0} aria-valuemax={100} aria-valuenow={status.progress}>
            <span style={{ width: `${status.progress}%` }} />
          </div>}
          {showProgress && progressVolume && <p className="progress-volume">{progressVolume}</p>}

          <div className="primary-zone">
            {status.phase === "needsGameData" && !gameDataPath && <button className="primary-action" onClick={selectData} disabled={requestPending}>{copy.chooseData}</button>}
            {status.phase === "needsGameData" && gameDataPath && <button className="primary-action" onClick={install} disabled={requestPending}>{copy.install}</button>}
            {status.phase === "ready" && <button className="primary-action" onClick={start} disabled={requestPending}>{copy.play}</button>}
            {status.phase === "running" && <button className="primary-action stop-action" onClick={stop} disabled={requestPending}>{copy.stop}</button>}
            {isBusy(status) && <button className="primary-action" disabled>{copy.wait}</button>}
            {status.phase === "error" && <button className="primary-action" onClick={() => void retry()} disabled={requestPending}>{copy.checkAgain}</button>}
            {status.phase === "error" && status.recoveryAvailable && <button className="secondary-action" onClick={() => void restoreRecovery()} disabled={requestPending}>{copy.restoreLast}</button>}
          </div>

          {status.phase === "needsGameData" && <button className="context-link" onClick={() => openPanel("settings")}>{gameDataPath ? copy.changeFolder : copy.installationOptions}</button>}
          {status.phase === "error" && <button className="context-link" onClick={() => openPanel("diagnostics")}>{copy.openDiagnostics}</button>}
        </section>

        {panel && <div className="panel-layer" onMouseDown={(event) => { if (event.target === event.currentTarget) closePanel(); }}>
          <section ref={panelRef} className="side-panel" role="dialog" aria-modal="true" aria-labelledby="panel-title">
            <header className="panel-header">
              {panel !== "settings" ? <button className="back-button" onClick={() => setPanel("settings")}>{copy.back}</button> : <span />}
              <h2 id="panel-title">{panelTitle}</h2>
              <button className="close-button" onClick={closePanel} aria-label={copy.close}>×</button>
            </header>

            <div className="panel-content">
              {panel === "settings" && <>
                <section className="settings-section">
                  <h3>{copy.language}</h3>
                  <div className="language-switch" aria-label={copy.language}>
                    <button aria-pressed={language === "fr"} className={language === "fr" ? "active" : ""} onClick={() => setLanguage("fr")}>Français</button>
                    <button aria-pressed={language === "en"} className={language === "en" ? "active" : ""} onClick={() => setLanguage("en")}>English</button>
                  </div>
                </section>

                {status.phase === "needsGameData" && <section className="settings-section install-settings">
                  <h3>{copy.installationOptions}</h3>
                  <label className="option-row"><input type="radio" name="client" checked={clientChoice === "managedOpenWow"} onChange={() => setClientChoice("managedOpenWow")} /><span><strong>{copy.managedClient}</strong><small>{copy.managedClientHelp}</small></span></label>
                  <label className={`option-row ${!status.originalClientSupported ? "muted" : ""}`}><input type="radio" name="client" checked={clientChoice === "originalWindows"} disabled={!status.originalClientSupported} onChange={() => setClientChoice("originalWindows")} /><span><strong>{copy.originalClient}</strong><small>{status.originalClientSupported ? copy.originalClientHelp : copy.originalUnavailable}</small></span></label>
                  <button className="secondary-action full" onClick={selectData} disabled={requestPending}>{gameDataPath ? copy.changeFolder : copy.chooseData}</button>
                  <p className={`helper ${gameDataError ? "error" : gameDataInspection ? "success" : ""}`}>{gameDataError ?? (gameDataInspection ? `Data ${gameDataInspection.locale} · build 12340` : copy.dataRequirement)}</p>
                  <label className="option-row"><input type="checkbox" checked={botsEnabled} onChange={(event) => { setBotsEnabled(event.target.checked); if (!event.target.checked) setAiEnabled(false); }} /><span><strong>{copy.populate}</strong><small>{copy.populateHelp}</small></span></label>
                  {botsEnabled && <label className="select-row"><span>{copy.worldProfile}</span>{profileSelect}</label>}
                  {botsEnabled && worldProfile === "custom" && <label className="select-row"><span>{copy.population}</span>{populationSelect}</label>}
                  <label className={`option-row ${aiCapability.state !== "recommended" ? "muted" : ""}`}><input type="checkbox" checked={aiEnabled} disabled={!botsEnabled || aiCapability.state !== "recommended"} onChange={(event) => setAiEnabled(event.target.checked)} /><span><strong>{copy.ai}</strong><small>{aiCapability.state === "checking" ? copy.aiChecking : aiCapability.state === "recommended" ? `${aiCapability.modelName} · ${aiCapability.downloadSizeGb ?? "?"} GB · ~${aiCapability.estimatedTokensPerSecond ?? "?"} tok/s` : copy.aiUnavailable}</small></span></label>
                </section>}

                {gameDataPath && status.phase !== "needsGameData" && <section className="settings-section game-folder-settings">
                  <h3>{copy.gameClient}</h3>
                  <div className="game-folder-path"><span>{copy.gameFolder}</span><code>{gameDataPath}</code></div>
                  <p className="helper">{status.clientChoice === "managedOpenWow" ? copy.managedPathHelp : copy.originalPathHelp}</p>
                  <button className="secondary-action full" onClick={changeInstalledGameData} disabled={requestPending || status.phase === "running"}>{copy.changeGameFolder}</button>
                  {(status.phase === "running" || gameDataError || gameDataFeedback) && <p className={`helper ${gameDataError ? "error" : gameDataFeedback ? "success" : ""}`}>{status.phase === "running" ? copy.stopToChangeGameFolder : gameDataError ?? gameDataFeedback}</p>}
                </section>}

                <nav className="settings-nav" aria-label={copy.settings}>
                  <button onClick={() => openPanel("companions")} disabled={!status.installed}><span><strong>{copy.companions}</strong><small>{copy.companionsBodyReady}</small></span></button>
                  <button onClick={() => openPanel("dialogues")} disabled={!status.installed}><span><strong>{copy.dialogues}</strong><small>{copy.dialoguesBody}</small></span></button>
                  <button onClick={() => openPanel("diagnostics")}><span><strong>{copy.diagnostics}</strong><small>{copy.diagnosticsBody}</small></span></button>
                </nav>

                {status.installed && status.accountName && <section className="account-details"><span>{copy.account}</span><strong>{status.accountName} / {status.accountPassword}</strong><small>{copy.accountHelp}</small></section>}
                <p className="build-info">{displayedPlatform}</p>
              </>}

              {panel === "companions" && <>
                <p className="panel-intro">{status.phase === "running" ? copy.companionsBodyRunning : copy.companionsBodyReady}</p>
                <label className="option-row"><input type="checkbox" checked={botsEnabled} onChange={(event) => setBotsEnabled(event.target.checked)} /><span><strong>{copy.populate}</strong><small>{copy.populateHelp}</small></span></label>
                {botsEnabled && <label className="select-row"><span>{copy.worldProfile}</span>{profileSelect}</label>}
                {botsEnabled && worldProfile === "custom" && <label className="select-row"><span>{copy.requestedPopulation}</span>{populationSelect}</label>}
                {botsEnabled && <dl className="facts-list"><div><dt>{copy.requestedPopulation}</dt><dd>{botCount}</dd></div><div><dt>{copy.appliedPopulation}</dt><dd>{status.appliedBotCount}</dd></div></dl>}
                <p className="helper">{copy.teamHelp}</p>
                {status.phase === "running" ? <button className="secondary-action full" onClick={applyPopulation} disabled={requestPending}>{copy.applyNow}</button> : <p className="helper">{copy.startToApply}</p>}
                {populationFeedback && <p className="success-message">{populationFeedback}</p>}
              </>}

              {panel === "dialogues" && <>
                <p className="panel-intro">{copy.dialoguesBody}</p>
                <div className="model-summary"><strong>{status.aiModel ?? aiCapability.modelName ?? (aiCapability.state === "checking" ? copy.aiChecking : copy.aiUnavailable)}</strong><p>{aiCapability.detail}</p></div>
                {(status.aiModel || aiCapability.state === "recommended") && <dl className="facts-list">
                  <div><dt>{copy.downloadSize}</dt><dd>{aiCapability.downloadSizeGb ? `${aiCapability.downloadSizeGb.toLocaleString(language, { maximumFractionDigits: 1 })} GB` : "—"}</dd></div>
                  <div><dt>{copy.diskAvailable}</dt><dd>{aiCapability.diskAvailableGb != null ? `${aiCapability.diskAvailableGb.toLocaleString(language, { maximumFractionDigits: 1 })} GB` : "—"}</dd></div>
                  <div><dt>{copy.estimatedSpeed}</dt><dd>{aiCapability.estimatedTokensPerSecond ? `~${aiCapability.estimatedTokensPerSecond} tok/s` : "—"}</dd></div>
                  <div><dt>{copy.modelLicense}</dt><dd>{aiCapability.modelLicense ?? "—"}</dd></div>
                </dl>}
                <p className="helper">{copy.dialogueLocalProof}</p>
                {status.aiModel && <label className="select-row"><span>{copy.chattiness}</span><select value={status.dialogueChattiness} disabled={requestPending || !status.aiEnabled} onChange={(event) => void changeChattiness(event.target.value as DialogueChattiness)}><option value="quiet">{copy.chattinessQuiet}</option><option value="balanced">{copy.chattinessBalanced}</option><option value="lively">{copy.chattinessLively}</option></select></label>}
                {status.phase === "running" ? <><p className="helper">{copy.closeToChange}</p><button className="secondary-action full" onClick={stop} disabled={requestPending}>{copy.stopForDialogues}</button></> : status.aiEnabled ?
                  <button className="secondary-action full" onClick={() => void configureDialogue(false)} disabled={requestPending}>{copy.deactivateDialogues}</button> :
                  <button className="secondary-action full" onClick={() => void configureDialogue(true)} disabled={requestPending || aiCapability.state !== "recommended" || aiCapability.diskSpaceSufficient === false || !botsEnabled}>{copy.activateDialogues}</button>}
                {aiCapability.diskSpaceSufficient === false && <p className="error-message">{copy.diskInsufficient}</p>}
                {dialogueFeedback && <p className="success-message">{dialogueFeedback}</p>}
                {dialogueError && <p className="error-message">{dialogueError}</p>}
              </>}

              {panel === "diagnostics" && <>
                <div className="diagnostic-actions"><p>{copy.diagnosticsBody}</p><button className="text-button" onClick={refreshDiagnostics} disabled={diagnosticPending}>{copy.refresh}</button></div>
                {diagnostics ? <>
                  <dl className="facts-list diagnostics-list"><div><dt>{copy.affectedComponent}</dt><dd>{({ client: copy.componentClient, database: copy.componentDatabase, server: copy.componentServer, bots: copy.componentBots, ai: copy.componentAi, launcher: copy.componentLauncher })[diagnostics.component]}</dd></div><div><dt>{copy.logsFolder}</dt><dd>{diagnostics.logsPath}</dd></div></dl>
                  <p className="diagnostic-summary">{diagnostics.summary}</p>
                  <div className="log-list">{diagnostics.recentEntries.length ? diagnostics.recentEntries.map((entry, index) => <code key={`${index}-${entry}`}>{entry}</code>) : <span>{copy.noRecentErrors}</span>}</div>
                  {status.phase === "error" && status.detail && <details><summary>{copy.cause}</summary><code>{status.detail}</code></details>}
                  <button className="secondary-action full" onClick={copyDiagnostics}>{copied ? copy.copied : copy.copy}</button>
                </> : <p className="helper">{copy.noDiagnostic}</p>}
              </>}
            </div>
          </section>
        </div>}
      </section>
    </main>
  );
}
