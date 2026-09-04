import { useEffect, useMemo, useRef, useState } from "react";
import { messages, preferredLanguage, type Copy, type Language } from "./i18n";
import realmIcon from "./assets/realmbox-icon.svg";
import SetupWizard from "./SetupWizard";
import { setupMessages } from "./setupCopy";
import {
  bootstrapLauncher,
  changeGameDataPath,
  chooseGameData,
  configureLocalDialogue,
  configureDialogueChattiness,
  configureSoloProfile,
  createRealmBackup,
  getRealmDiagnostics,
  inspectAiCapability,
  inspectGameData,
  inspectRealmBackup,
  inspectSoloProfiles,
  installRealm,
  queryLocalGuide,
  restoreLastRecovery,
  rollbackSoloProfile,
  startRealm,
  stopRealm,
  subscribeLauncherProgress,
  subscribeLauncherStatus,
  updatePlayerbotPopulation,
} from "./runtime";
import type { AiCapability, BotPresence, ClientChoice, DialogueChattiness, GameDataInspection, LauncherCommandError, LauncherErrorCode, LauncherStatus, LocalGuideKind, LocalGuideResponse, RealmBackupSummary, RealmDiagnostics, SoloProfile, SoloProfileView } from "./types";

type Panel = "settings" | "companions" | "dialogues" | "backups" | "guide" | "solo" | "diagnostics";
type WorldProfile = "quiet" | "balanced" | "dense" | "custom";

const initialStatus: LauncherStatus = {
  phase: "checking", message: "Vérification de l’installation…", detail: null, errorCode: null, progress: 0,
  installed: false, recoveryAvailable: false, botsEnabled: true, botCount: 50, requestedBotCount: 50, appliedBotCount: 50, aiEnabled: false, aiModel: null,
  botPresence: "natural", dialogueChattiness: "balanced",
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

function formatBackupDate(timestamp: number, language: Language) {
  return new Intl.DateTimeFormat(language, { dateStyle: "medium", timeStyle: "short" }).format(timestamp);
}

function phaseCopy(status: LauncherStatus, copy: Copy) {
  if (status.phase === "running") return { title: copy.runningTitle, body: copy.runningBody };
  if (status.phase === "ready") return {
    title: copy.readyTitle,
    body: status.message.startsWith("Les ressources Docker seront reconstruites") ? localizedOperation(status.message, copy) : copy.readyBody,
  };
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
    "Préparation sécurisée du serveur": "Preparing the server update safely",
    "Téléchargement du serveur épinglé": "Downloading the pinned server",
    "Installation du module Playerbots": "Installing Playerbots",
    "Ajout des dialogues locaux": "Adding local dialogue",
    "Téléchargement du moteur de dialogue": "Downloading the local dialogue engine",
    "Téléchargement du serveur local": "Downloading the local server",
    "Construction du serveur local": "Building the local server",
    "Préparation de la base locale": "Preparing the local save",
    "Sauvegarde des personnages": "Backing up characters",
    "Les ressources Docker seront reconstruites depuis la sauvegarde locale vérifiée au prochain lancement": "Docker resources will be rebuilt from the verified local backup when you play",
    "Reconstruction des ressources Docker": "Rebuilding Docker resources",
    "Création du compte local": "Creating the local account",
    "Préparation du modèle local": "Preparing the local model",
    "Finalisation de RealmBox": "Finishing RealmBox setup",
    "Mise à jour du serveur local": "Updating the local server",
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
  const setupCopy = setupMessages[language];
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
  const [botPresence, setBotPresence] = useState<BotPresence>("natural");
  const [worldProfile, setWorldProfile] = useState<WorldProfile>("balanced");
  const [clientChoice, setClientChoice] = useState<ClientChoice>("managedOpenWow");
  const [aiEnabled, setAiEnabled] = useState(false);
  const [aiCapability, setAiCapability] = useState<AiCapability>(checkingAi);
  const [requestPending, setRequestPending] = useState(false);
  const [populationFeedback, setPopulationFeedback] = useState<string | null>(null);
  const [dialogueFeedback, setDialogueFeedback] = useState<string | null>(null);
  const [dialogueError, setDialogueError] = useState<string | null>(null);
  const [backupSummary, setBackupSummary] = useState<RealmBackupSummary | null>(null);
  const [backupLoaded, setBackupLoaded] = useState(false);
  const [backupPending, setBackupPending] = useState(false);
  const [backupFeedback, setBackupFeedback] = useState<string | null>(null);
  const [backupError, setBackupError] = useState<string | null>(null);
  const [guideKind, setGuideKind] = useState<LocalGuideKind>("quest");
  const [guideTerm, setGuideTerm] = useState("");
  const [guideResponse, setGuideResponse] = useState<LocalGuideResponse | null>(null);
  const [guidePending, setGuidePending] = useState(false);
  const [guideError, setGuideError] = useState(false);
  const guideRequest = useRef(0);
  const [soloView, setSoloView] = useState<SoloProfileView | null>(null);
  const [soloSelected, setSoloSelected] = useState<SoloProfile>("normal");
  const [soloPending, setSoloPending] = useState(false);
  const [soloError, setSoloError] = useState(false);
  const [soloFeedback, setSoloFeedback] = useState<string | null>(null);
  const [diagnostics, setDiagnostics] = useState<RealmDiagnostics | null>(null);
  const [diagnosticPending, setDiagnosticPending] = useState(false);
  const [copied, setCopied] = useState(false);
  const bootstrapRequest = useRef<Promise<LauncherStatus> | null>(null);
  const aiRequest = useRef<Promise<AiCapability> | null>(null);

  function applyLauncherStatus(next: LauncherStatus) {
    setStatus(next);
    // A retry of first-time setup must not discard the player's validated draft.
    // Installed-state readback, on the other hand, remains authoritative.
    if (!next.installed) return;
    setBotsEnabled(next.botsEnabled);
    setBotCount(next.requestedBotCount);
    setBotPresence(next.botPresence);
    setWorldProfile(profileForPopulation(next.requestedBotCount));
    setAiEnabled(next.aiEnabled);
    setClientChoice(next.clientChoice);
    setGameDataPath(next.gameDataPath);
  }

  useEffect(() => {
    let active = true;
    let unlistenProgress: () => void = () => undefined;
    let unlistenStatus: () => void = () => undefined;
    void subscribeLauncherProgress((progress) => active && setStatus((current) => ({ ...current, ...progress })))
      .then((unlisten) => { unlistenProgress = unlisten; });
    void subscribeLauncherStatus((next) => {
      if (!active) return;
      applyLauncherStatus(next);
    }).then((unlisten) => { unlistenStatus = unlisten; });
    bootstrapRequest.current ??= bootstrapLauncher();
    void bootstrapRequest.current.then((next) => {
      if (!active) return;
      applyLauncherStatus(next);
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
  const dialogueActivationModel = status.aiModel ?? aiCapability.ollamaModel;
  const capabilityMatchesInstalledModel = status.aiModel != null && aiCapability.ollamaModel === status.aiModel;

  async function selectData() {
    setRequestPending(true); setGameDataError(null);
    try {
      const selected = await chooseGameData(language);
      if (!selected) return;
      const inspection = await inspectGameData(selected);
      setGameDataInspection(inspection); setGameDataPath(inspection.path);
      setStatus((current) => ({ ...current, detail: null, errorCode: null }));
    } catch (error) {
      setGameDataInspection(null); setGameDataPath(null); setGameDataError(String(error));
      setStatus((current) => ({ ...current, ...commandError(error) }));
    } finally { setRequestPending(false); }
  }

  async function install() {
    if (requestPending || status.installed || status.phase !== "needsGameData" || !gameDataPath || gameDataInspection?.path !== gameDataPath) return;
    setRequestPending(true);
    try { applyLauncherStatus(await installRealm(gameDataPath, clientChoice, botsEnabled, botCount, botPresence, aiEnabled, aiEnabled ? aiCapability.ollamaModel : null)); }
    catch (error) { setStatus((current) => ({ ...current, phase: "error", ...commandError(error) })); }
    finally { setRequestPending(false); }
  }

  async function changeInstalledGameData() {
    setRequestPending(true); setGameDataError(null); setGameDataFeedback(null);
    try {
      const selected = await chooseGameData(language);
      if (!selected) return;
      const inspection = await inspectGameData(selected);
      const next = await changeGameDataPath(inspection.path);
      applyLauncherStatus(next); setGameDataInspection(inspection);
      setGameDataFeedback(copy.gameFolderUpdated);
    } catch (error) {
      setGameDataFeedback(null); setGameDataError(String(error));
    } finally { setRequestPending(false); }
  }

  async function start() {
    setRequestPending(true);
    try { applyLauncherStatus(await startRealm(botsEnabled, botCount, botPresence, aiEnabled)); }
    catch (error) { setStatus((current) => ({ ...current, phase: "error", ...commandError(error) })); }
    finally { setRequestPending(false); }
  }

  async function stop() {
    setRequestPending(true);
    try { applyLauncherStatus(await stopRealm()); }
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
    try { applyLauncherStatus(await bootstrapLauncher()); }
    catch (error) { setStatus((current) => ({ ...current, phase: "error", ...commandError(error) })); }
    finally { setRequestPending(false); }
  }

  async function restoreRecovery() {
    setRequestPending(true);
    try { applyLauncherStatus(await restoreLastRecovery()); }
    catch (error) { setStatus((current) => ({ ...current, phase: "error", ...commandError(error) })); }
    finally { setRequestPending(false); }
  }

  async function applyPopulation() {
    setRequestPending(true); setPopulationFeedback(null);
    try {
      const next = await updatePlayerbotPopulation(botsEnabled, botCount, botPresence);
      applyLauncherStatus(next);
      setPopulationFeedback(next.phase === "running" ? copy.applied : copy.savedForNextLaunch);
    } catch (error) {
      setStatus((current) => ({ ...current, phase: "error", ...commandError(error) })); setPanel(null);
    } finally { setRequestPending(false); }
  }

  async function configureDialogue(enabled: boolean) {
    if (enabled && !dialogueActivationModel) return;
    const previousPhase = status.phase;
    setRequestPending(true); setDialogueFeedback(null); setDialogueError(null);
    try {
      const next = await configureLocalDialogue(enabled, enabled ? dialogueActivationModel : status.aiModel);
      applyLauncherStatus(next);
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
      applyLauncherStatus(next); setDialogueFeedback(copy.chattinessSaved);
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

  async function refreshBackup() {
    setBackupPending(true); setBackupError(null);
    try { setBackupSummary(await inspectRealmBackup()); }
    catch { setBackupSummary(null); setBackupError(copy.backupFailed); }
    finally { setBackupLoaded(true); setBackupPending(false); }
  }

  async function createBackup() {
    setBackupPending(true); setBackupFeedback(null); setBackupError(null);
    try {
      setBackupSummary(await createRealmBackup());
      setBackupLoaded(true); setBackupFeedback(copy.backupCreated);
    } catch {
      setBackupError(copy.backupFailed);
    } finally { setBackupPending(false); }
  }

  async function searchGuide() {
    if (guidePending || guideTerm.trim().length < 2 || Array.from(guideTerm).length > 64) return;
    const request = ++guideRequest.current;
    setGuidePending(true); setGuideResponse(null); setGuideError(false);
    try {
      const response = await queryLocalGuide(guideKind, guideTerm, language === "fr" ? "frFR" : "enUS");
      if (request === guideRequest.current) setGuideResponse(response);
    } catch {
      if (request === guideRequest.current) setGuideError(true);
    } finally {
      if (request === guideRequest.current) setGuidePending(false);
    }
  }

  function changeLanguage(next: Language) {
    setLanguage(next);
    guideRequest.current += 1;
    setGuidePending(false); setGuideResponse(null); setGuideError(false);
  }

  async function refreshSolo() {
    setSoloPending(true); setSoloError(false); setSoloFeedback(null);
    try {
      const view = await inspectSoloProfiles();
      setSoloView(view); setSoloSelected(view.activeProfile ?? "normal");
    } catch { setSoloView(null); setSoloError(true); }
    finally { setSoloPending(false); }
  }

  async function applySolo(rollback = false) {
    if (status.phase === "running" || isBusy(status)) return;
    setSoloPending(true); setRequestPending(true); setSoloError(false); setSoloFeedback(null);
    try {
      const view = rollback ? await rollbackSoloProfile() : await configureSoloProfile(soloSelected);
      setSoloView(view); setSoloSelected(view.activeProfile ?? "normal");
      setSoloFeedback(rollback ? copy.soloRestored : copy.soloSaved);
    } catch {
      setSoloError(true);
      // A durable journal may still need finalizing after the config changed.
      // Read back the actual state instead of presenting a fictitious rollback.
      try {
        const view = await inspectSoloProfiles();
        setSoloView(view); setSoloSelected(view.activeProfile ?? "normal");
      } catch { setSoloView(null); }
    }
    finally { setSoloPending(false); setRequestPending(false); }
  }

  function openPanel(next: Panel) {
    if (!panel && document.activeElement instanceof HTMLElement) panelTriggerRef.current = document.activeElement;
    setPanel(next);
    if (next === "diagnostics") void refreshDiagnostics();
    if (next === "backups") void refreshBackup();
    if (next === "solo") void refreshSolo();
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
  const presenceChoices = (
    <fieldset className="choice-group" aria-label={copy.presence}>
      <legend>{copy.presence}</legend>
      {([
        ["dispersed", copy.presenceDispersed, copy.presenceDispersedHelp],
        ["natural", copy.presenceNatural, copy.presenceNaturalHelp],
        ["close", copy.presenceClose, copy.presenceCloseHelp],
      ] as const).map(([value, label, help]) => <label className="choice-card" key={value}>
        <input type="radio" name="bot-presence" value={value} checked={botPresence === value} onChange={() => { setBotPresence(value); setPopulationFeedback(null); }} />
        <span><strong>{label}{value === "natural" && <em>{copy.recommended}</em>}</strong><small>{help}</small></span>
      </label>)}
    </fieldset>
  );

  const panelTitle = panel === "companions" ? copy.companionsTitle : panel === "dialogues" ? copy.dialoguesTitle : panel === "backups" ? copy.backupsTitle : panel === "guide" ? copy.guideTitle : panel === "solo" ? copy.soloTitle : panel === "diagnostics" ? copy.diagnosticsTitle : copy.settings;
  const selectedSoloProfile = soloView?.profiles.find((profile) => profile.profile === soloSelected);
  const soloValue = (key: string) => selectedSoloProfile?.settings.find((setting) => setting.key === key)?.value ?? "—";
  const activeSoloProfile = soloView?.profiles.find((profile) => profile.profile === soloView.activeProfile);
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

        <div className="launcher-content" inert={panel !== null} aria-hidden={panel !== null || undefined}>

        <div className="launcher-brand" aria-label="RealmBox">
          <img src={realmIcon} alt="" />
          <span>RealmBox</span>
        </div>

        <button className="settings-button" onClick={() => openPanel("settings")}>{copy.settings}</button>

        {status.phase === "needsGameData" && !status.installed ? <SetupWizard
          language={language} gameDataPath={gameDataPath} inspection={gameDataInspection} gameDataError={gameDataError}
          busy={requestPending} selectData={selectData} install={install}
          openDiagnostics={() => openPanel("diagnostics")}
          clientChoice={clientChoice} setClientChoice={setClientChoice} originalClientSupported={status.originalClientSupported}
          botsEnabled={botsEnabled} botCount={botCount} botPresence={botPresence}
          aiEnabled={aiEnabled} setAiEnabled={setAiEnabled} capability={aiCapability}
          worldControls={<>
            <label className="option-row"><input type="checkbox" checked={botsEnabled} onChange={(event) => { setBotsEnabled(event.target.checked); if (!event.target.checked) setAiEnabled(false); }} /><span><strong>{copy.populate}</strong><small>{copy.populateHelp}</small></span></label>
            {botsEnabled && <>
              <fieldset className="choice-group population-cards"><legend>{copy.worldProfile}</legend>
                {([
                  [25, copy.quietProfile], [50, copy.balancedProfile], [100, copy.denseProfile],
                ] as const).map(([count, label]) => <label key={count} className="choice-card"><input type="radio" name="setup-population" checked={botCount === count && worldProfile !== "custom"} onChange={() => { setBotCount(count); setWorldProfile(profileForPopulation(count)); }} /><span><strong>{label}</strong><small>{count} {setupCopy.bots}</small></span></label>)}
                <label className="choice-card"><input type="radio" name="setup-population" checked={worldProfile === "custom"} onChange={() => setWorldProfile("custom")} /><span><strong>{copy.customProfile}</strong><small>5 – 150 {setupCopy.bots}</small></span></label>
              </fieldset>
              {worldProfile === "custom" && <label className="select-row"><span>{copy.population}</span>{populationSelect}</label>}
              <p className="helper">{copy.populationHelp}</p>
              {presenceChoices}
            </>}
          </>}
        /> : <section className="status-card" aria-live="polite">
          <div className={`status-marker ${status.phase}`} aria-hidden="true" />
          <h1>{status.phase === "error" ? presentedError.cause : phase.title}</h1>
          <p>{status.phase === "error" ? presentedError.recovery : phase.body}</p>

          {showProgress && <div className="active-progress" role="progressbar" aria-label={copy.progress} aria-valuemin={0} aria-valuemax={100} aria-valuenow={status.progress}>
            <span style={{ width: `${status.progress}%` }} />
          </div>}
          {showProgress && progressVolume && <p className="progress-volume">{progressVolume}</p>}

          <div className="primary-zone">
            {status.phase === "ready" && <button className="primary-action" onClick={start} disabled={requestPending}>{copy.play}</button>}
            {status.phase === "running" && <button className="primary-action stop-action" onClick={stop} disabled={requestPending}>{copy.stop}</button>}
            {isBusy(status) && <button className="primary-action" disabled>{copy.wait}</button>}
            {status.phase === "error" && <button className="primary-action" onClick={() => void retry()} disabled={requestPending}>{copy.checkAgain}</button>}
            {status.phase === "error" && status.recoveryAvailable && <button className="secondary-action" onClick={() => void restoreRecovery()} disabled={requestPending}>{copy.restoreLast}</button>}
          </div>

          {status.phase === "error" && <button className="context-link" onClick={() => openPanel("diagnostics")}>{copy.openDiagnostics}</button>}
          {status.installed && ["ready", "running"].includes(status.phase) && status.accountName && <details className="login-help"><summary>{setupCopy.login}</summary><p>{setupCopy.afterPlay}</p><strong>{status.accountName} / {status.accountPassword}</strong><p>{copy.accountHelp}</p></details>}
        </section>}

        {status.phase === "installing" && <aside className="installation-followup" aria-label={setupCopy.installationDetail}>
          <h2>{setupCopy.currentStep}</h2>
          <strong>{status.component ? ({ gameData: setupCopy.prepareData, client: setupCopy.prepareClient, server: setupCopy.prepareWorld, bots: setupCopy.prepareWorld, database: setupCopy.prepareSave, ai: setupCopy.prepareAi, launcher: setupCopy.prepareFinish })[status.component] : phase.body}</strong>
          <p>{setupCopy.progressBody}</p><p>{setupCopy.setupTime}</p>
          <button className="text-button" onClick={() => openPanel("diagnostics")}>{copy.openDiagnostics}</button>
        </aside>}

        {status.installed && ["ready", "running"].includes(status.phase) && <aside className="realm-dashboard" aria-label={setupCopy.manage}>
          <header><h2>{setupCopy.manage}</h2><p>{setupCopy.manageBody}</p></header>
          <dl className="realm-overview"><div><dt>{setupCopy.configured}</dt><dd>{status.botsEnabled ? `${status.appliedBotCount} ${setupCopy.bots}` : copy.off}</dd></div><div><dt>{copy.presence}</dt><dd>{status.botsEnabled ? { dispersed: copy.presenceDispersed, natural: copy.presenceNatural, close: copy.presenceClose }[status.botPresence] : "—"}</dd></div><div><dt>{copy.ai}</dt><dd>{status.aiEnabled ? setupCopy.enabled : copy.off}</dd></div></dl>
          <p className="helper">{setupCopy.limitHelp}</p>
          <nav aria-label={setupCopy.manage}>
            <button onClick={() => openPanel("companions")}>{setupCopy.shortcutsBots}<span aria-hidden="true">›</span></button>
            <button onClick={() => openPanel("dialogues")}>{copy.dialogues}<span aria-hidden="true">›</span></button>
            <button onClick={() => openPanel("solo")}>{setupCopy.shortcutsSolo}<span aria-hidden="true">›</span></button>
            <button onClick={() => openPanel("backups")}>{setupCopy.shortcutsBackup}<span aria-hidden="true">›</span></button>
            <button onClick={() => openPanel("guide")}>{setupCopy.shortcutsGuide}<span aria-hidden="true">›</span></button>
          </nav>
        </aside>}

        </div>

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
                    <button aria-pressed={language === "fr"} className={language === "fr" ? "active" : ""} onClick={() => changeLanguage("fr")}>Français</button>
                    <button aria-pressed={language === "en"} className={language === "en" ? "active" : ""} onClick={() => changeLanguage("en")}>English</button>
                  </div>
                </section>

                {status.phase === "needsGameData" && <p className="helper">{setupCopy.worldBody}</p>}

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
                  <button onClick={() => openPanel("solo")} disabled={!status.installed}><span><strong>{copy.solo}</strong><small>{copy.soloNavBody}</small></span></button>
                  <button onClick={() => openPanel("backups")} disabled={!status.installed}><span><strong>{copy.backups}</strong><small>{copy.backupsNavBody}</small></span></button>
                  <button onClick={() => openPanel("guide")} disabled={!status.installed}><span><strong>{copy.guide}</strong><small>{copy.guideNavBody}</small></span></button>
                  <button onClick={() => openPanel("diagnostics")}><span><strong>{copy.diagnostics}</strong><small>{copy.diagnosticsBody}</small></span></button>
                </nav>

                {status.installed && status.accountName && <section className="account-details"><span>{copy.account}</span><strong>{status.accountName} / {status.accountPassword}</strong><small>{copy.accountHelp}</small></section>}
                <p className="build-info">{displayedPlatform}</p>
              </>}

              {panel === "companions" && <>
                <p className="panel-intro">{status.phase === "running" ? copy.companionsBodyRunning : copy.companionsBodyReady}</p>
                <label className="option-row"><input type="checkbox" checked={botsEnabled} onChange={(event) => { setBotsEnabled(event.target.checked); if (!event.target.checked) setAiEnabled(false); setPopulationFeedback(null); }} /><span><strong>{copy.populate}</strong><small>{copy.populateHelp}</small></span></label>
                {botsEnabled && <label className="select-row"><span>{copy.worldProfile}</span>{profileSelect}</label>}
                {botsEnabled && worldProfile === "custom" && <label className="select-row"><span>{copy.requestedPopulation}</span>{populationSelect}</label>}
                {botsEnabled && presenceChoices}
                {botsEnabled && <dl className="facts-list"><div><dt>{copy.requestedPopulation}</dt><dd>{botCount}</dd></div><div><dt>{copy.appliedPopulation}</dt><dd>{status.appliedBotCount}</dd></div></dl>}
                {botsEnabled && <section className="mode-summary" aria-labelledby="party-behavior-title">
                  <h3 id="party-behavior-title">{copy.partyBehavior}</h3>
                  <p>{copy.teamHelp}</p>
                  <ul><li><strong>{copy.behaviorEscort}</strong> — {copy.behaviorEscortHelp}</li><li><strong>{copy.behaviorGuard}</strong> — {copy.behaviorGuardHelp}</li><li><strong>{copy.behaviorFree}</strong> — {copy.behaviorFreeHelp}</li></ul>
                </section>}
                {!botsEnabled && <p className="helper">{copy.behaviorRequiresCompanions}</p>}
                <button className="secondary-action full" onClick={applyPopulation} disabled={requestPending}>{status.phase === "running" ? copy.applyNow : copy.saveForNextLaunch}</button>
                {populationFeedback && <p className="success-message" role="status">{populationFeedback}</p>}
              </>}

              {panel === "dialogues" && <>
                <p className="panel-intro">{copy.dialoguesBody}</p>
                <div className="model-summary"><strong>{status.aiModel ?? aiCapability.modelName ?? (aiCapability.state === "checking" ? copy.aiChecking : copy.aiUnavailable)}</strong><p>{status.aiModel ? copy.modelInstalled : aiCapability.detail}</p></div>
                {(aiCapability.state === "recommended" && (!status.aiModel || capabilityMatchesInstalledModel)) && <dl className="facts-list">
                  <div><dt>{copy.downloadSize}</dt><dd>{aiCapability.downloadSizeGb ? `${aiCapability.downloadSizeGb.toLocaleString(language, { maximumFractionDigits: 1 })} GB` : "—"}</dd></div>
                  <div><dt>{copy.diskAvailable}</dt><dd>{aiCapability.diskAvailableGb != null ? `${aiCapability.diskAvailableGb.toLocaleString(language, { maximumFractionDigits: 1 })} GB` : "—"}</dd></div>
                  <div><dt>{copy.estimatedSpeed}</dt><dd>{aiCapability.estimatedTokensPerSecond ? `~${aiCapability.estimatedTokensPerSecond} tok/s` : "—"}</dd></div>
                  <div><dt>{copy.modelLicense}</dt><dd>{aiCapability.modelLicense ?? "—"}</dd></div>
                </dl>}
                <p className="helper">{copy.dialogueLocalProof}</p>
                {status.aiModel && <fieldset className="choice-group" aria-label={copy.chattiness} disabled={requestPending || (status.phase === "running" && !status.aiEnabled)}>
                  <legend>{copy.chattiness}</legend>
                  {([
                    ["quiet", copy.chattinessQuiet],
                    ["balanced", copy.chattinessBalanced],
                    ["lively", copy.chattinessLively],
                  ] as const).map(([value, label]) => <label className="choice-card" key={value}>
                    <input type="radio" name="dialogue-profile" value={value} checked={status.dialogueChattiness === value} onChange={() => void changeChattiness(value as DialogueChattiness)} />
                    <span><strong>{label}{value === "balanced" && <em>{copy.recommended}</em>}</strong></span>
                  </label>)}
                </fieldset>}
                {status.phase === "running" ? <><p className="helper">{copy.closeToChange}</p><button className="secondary-action full" onClick={stop} disabled={requestPending}>{copy.stopForDialogues}</button></> : status.aiEnabled ?
                  <button className="secondary-action full" onClick={() => void configureDialogue(false)} disabled={requestPending}>{copy.deactivateDialogues}</button> :
                  <button className="secondary-action full" onClick={() => void configureDialogue(true)} disabled={requestPending || !botsEnabled || (!status.aiModel && (aiCapability.state !== "recommended" || aiCapability.diskSpaceSufficient === false))}>{status.aiModel ? copy.reactivateDialogues : copy.activateDialogues}</button>}
                {!botsEnabled && <p className="error-message" role="alert">{copy.dialoguesRequireCompanions}</p>}
                {!status.aiModel && aiCapability.diskSpaceSufficient === false && <p className="error-message" role="alert">{copy.diskInsufficient}</p>}
                {dialogueFeedback && <p className="success-message" role="status">{dialogueFeedback}</p>}
                {dialogueError && <p className="error-message" role="alert">{dialogueError}</p>}
              </>}

              {panel === "backups" && <>
                <p className="panel-intro">{copy.backupsBody}</p>
                {backupPending && !backupLoaded && <p className="helper">{copy.backupChecking}</p>}
                {backupLoaded && backupSummary && <dl className="facts-list">
                  <div><dt>{copy.latestBackup}</dt><dd>{formatBackupDate(backupSummary.createdAtUnixMs, language)}</dd></div>
                  <div><dt>{copy.backupSize}</dt><dd>{formatBytes(backupSummary.sizeBytes, language)}</dd></div>
                  <div><dt>{copy.integrity}</dt><dd>{copy.verified}</dd></div>
                </dl>}
                {backupLoaded && !backupSummary && !backupError && <div className="model-summary"><strong>{copy.noBackup}</strong><p>{copy.backupFirstHelp}</p></div>}
                <p className="helper">{status.phase === "running" ? copy.backupRunningHelp : copy.backupLocalHelp}</p>
                <button className="secondary-action full" onClick={() => void createBackup()} disabled={backupPending}>{backupPending ? copy.backupWorking : copy.backupNow}</button>
                {backupFeedback && <p className="success-message" role="status">{backupFeedback}</p>}
                {backupError && <p className="error-message" role="alert">{backupError}</p>}
              </>}

              {panel === "solo" && <>
                <p className="panel-intro">{copy.soloBody}</p>
                {soloPending && !soloView && <p className="helper">{copy.wait}</p>}
                {soloView && <>
                  <div className="model-summary"><p>{copy.soloCurrent}</p><strong>{activeSoloProfile ? language === "fr" ? activeSoloProfile.labelFr : activeSoloProfile.labelEn : copy.soloCustom}</strong></div>
                  <fieldset className="choice-group" disabled={soloPending || status.phase === "running" || isBusy(status)}>
                    <legend>{copy.soloSelection}</legend>
                    {soloView.profiles.map((profile) => <label className="choice-card" key={profile.profile}><input type="radio" name="solo-profile" checked={soloSelected === profile.profile} onChange={() => { setSoloSelected(profile.profile); setSoloFeedback(null); setSoloError(false); }} /><span><strong>{language === "fr" ? profile.labelFr : profile.labelEn}</strong></span></label>)}
                  </fieldset>
                  <dl className="facts-list">
                    <div><dt>{copy.soloXp}</dt><dd>×{soloValue("Rate.XP.Kill")}</dd></div>
                    <div><dt>{copy.soloReputation}</dt><dd>×{soloValue("Rate.Reputation.Gain")}</dd></div>
                    <div><dt>{copy.soloMoney}</dt><dd>×{soloValue("Rate.Drop.Money")}</dd></div>
                    <div><dt>{copy.soloProfessions}</dt><dd>{soloValue("MaxPrimaryTradeSkill")}</dd></div>
                    <div><dt>{copy.soloLevelRequirement}</dt><dd>{soloValue("Instance.IgnoreLevel") === "1" ? copy.soloRelaxed : copy.soloStandard}</dd></div>
                    <div><dt>{copy.soloRaidGroup}</dt><dd>{soloValue("Instance.IgnoreRaid") === "1" ? copy.soloRelaxed : copy.soloStandard}</dd></div>
                    <div><dt>{copy.soloQuestsInRaid}</dt><dd>{soloValue("Quests.IgnoreRaid") === "1" ? copy.soloAllowed : copy.soloStandard}</dd></div>
                  </dl>
                  <p className="helper">{copy.soloXpHelp}</p>
                  <p className="helper">{copy.soloWarning}</p>
                  {soloView.pendingChange && <p className="helper">{copy.soloPendingChange}</p>}
                  {status.phase === "running" ? <><p className="helper">{copy.soloStop}</p><button className="secondary-action full" onClick={stop} disabled={requestPending}>{copy.soloStopAction}</button></> : <>
                    <button className="secondary-action full" onClick={() => void applySolo()} disabled={soloPending || isBusy(status)}>{copy.soloApply}</button>
                    <button className="secondary-action full" onClick={() => void applySolo(true)} disabled={soloPending || isBusy(status) || !soloView.rollbackAvailable}>{copy.soloRollback}</button>
                  </>}
                </>}
                {soloError && <p className="error-message" role="alert">{copy.soloFailed}</p>}
                {soloFeedback && <p className="success-message" role="status">{soloFeedback}</p>}
              </>}

              {panel === "guide" && <>
                <p className="panel-intro">{copy.guideBody}</p>
                <form onSubmit={(event) => { event.preventDefault(); void searchGuide(); }}>
                  <label className="select-row"><span>{copy.guideKind}</span><select value={guideKind} disabled={guidePending} onChange={(event) => { setGuideKind(event.target.value as LocalGuideKind); setGuideResponse(null); setGuideError(false); }}><option value="quest">{copy.guideQuests}</option><option value="item">{copy.guideItems}</option></select></label>
                  <label className="guide-search"><span>{copy.guideSearchLabel}</span><input type="search" value={guideTerm} minLength={2} maxLength={64} disabled={guidePending} onChange={(event) => { setGuideTerm(event.target.value); setGuideResponse(null); setGuideError(false); }} aria-describedby="guide-search-help" /></label>
                  <p id="guide-search-help" className="helper">{copy.guideSearchHelp}</p>
                  <button className="secondary-action full" type="submit" disabled={guidePending || guideTerm.trim().length < 2}>{guidePending ? copy.guideSearching : copy.guideSearch}</button>
                </form>
                <div className="guide-results" aria-live="polite" aria-busy={guidePending}>
                  {(guideError || guideResponse?.uncertainty === "unavailable") && <p className="error-message" role="alert">{copy.guideUnavailable}</p>}
                  {guideResponse?.uncertainty === "partial" && <p className="helper">{copy.guidePartial}</p>}
                  {guideResponse && guideResponse.uncertainty !== "unavailable" && guideResponse.entries.length === 0 && <p className="helper">{copy.guideEmpty}</p>}
                  {guideResponse?.entries.map((entry) => <article className="guide-entry" key={entry.id}>
                    <h3>{entry.title}</h3>
                    {entry.metadata.level != null && <p className="guide-level">{guideKind === "quest" ? copy.guideQuestLevel : copy.guideItemLevel} {entry.metadata.level} · #{entry.id}</p>}
                    <p>{entry.summary || copy.guideNoDescription}</p>
                  </article>)}
                  {guideResponse?.provenance && <p className="helper">{copy.guideSource}{guideResponse.provenance.observedAtUnixMs != null ? ` · ${formatBackupDate(guideResponse.provenance.observedAtUnixMs, language)}` : ""}</p>}
                </div>
              </>}

              {panel === "diagnostics" && <>
                <div className="diagnostic-actions"><p>{copy.diagnosticsBody}</p><button className="text-button" onClick={refreshDiagnostics} disabled={diagnosticPending}>{copy.refresh}</button></div>
                {(status.phase === "error" || gameDataError) && status.detail && <details><summary>{copy.cause}</summary><code>{status.detail}</code></details>}
                {diagnostics ? <>
                  <dl className="facts-list diagnostics-list"><div><dt>{copy.affectedComponent}</dt><dd>{({ client: copy.componentClient, database: copy.componentDatabase, server: copy.componentServer, bots: copy.componentBots, ai: copy.componentAi, launcher: copy.componentLauncher })[diagnostics.component]}</dd></div><div><dt>{copy.logsFolder}</dt><dd>{diagnostics.logsPath}</dd></div></dl>
                  <p className="diagnostic-summary">{diagnostics.summary}</p>
                  <div className="log-list">{diagnostics.recentEntries.length ? diagnostics.recentEntries.map((entry, index) => <code key={`${index}-${entry}`}>{entry}</code>) : <span>{copy.noRecentErrors}</span>}</div>
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
