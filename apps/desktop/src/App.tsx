import { useEffect, useMemo, useRef, useState } from "react";
import {
  bootstrapLauncher,
  chooseGameData,
  inspectAiCapability,
  inspectGameData,
  installRealm,
  startRealm,
  stopRealm,
  subscribeLauncherProgress,
  subscribeLauncherStatus,
} from "./runtime";
import type { AiCapability, ClientChoice, GameDataInspection, LauncherComponent, LauncherStatus } from "./types";

const initialStatus: LauncherStatus = {
  phase: "checking",
  message: "Vérification de l’installation…",
  detail: null,
  progress: 0,
  installed: false,
  botsEnabled: true,
  aiEnabled: false,
  aiModel: null,
  gameDataPath: null,
  accountName: null,
  accountPassword: null,
  clientChoice: "managedOpenWow",
  originalClientSupported: false,
  platformLabel: "Détection en cours",
  components: [],
};

const checkingAi: AiCapability = {
  state: "checking",
  deviceName: null,
  ramGb: null,
  modelId: null,
  modelName: null,
  ollamaModel: null,
  grade: null,
  estimatedTokensPerSecond: null,
  detail: "CanIRun évalue la mémoire disponible pour le jeu et les dialogues…",
  sourceUrl: "https://www.canirun.ai/",
};

function isBusy(status: LauncherStatus) {
  return ["checking", "installing", "starting", "stopping"].includes(status.phase);
}

function setupComponent(
  component: LauncherComponent,
  status: LauncherStatus,
  botsEnabled: boolean,
  aiEnabled: boolean,
  aiCapability: AiCapability,
  clientChoice: ClientChoice,
): LauncherComponent {
  if (status.phase !== "needsGameData") return component;
  if (component.id === "client") return {
    ...component,
    detail: clientChoice === "managedOpenWow" ? "OpenWoW sera téléchargé et vérifié" : "Le client du dossier choisi sera utilisé",
  };
  if (component.id === "bots") return {
    ...component,
    state: botsEnabled ? "missing" : "stopped",
    detail: botsEnabled ? "Choisis pour l’installation" : "Désactivés par le joueur",
  };
  if (component.id === "ai") return {
    ...component,
    state: aiEnabled ? "missing" : "stopped",
    detail: aiEnabled ? `${aiCapability.modelName ?? "Modèle local"} · choisi` : "Désactivés par le joueur",
  };
  return component;
}

export default function App() {
  const [status, setStatus] = useState(initialStatus);
  const [gameDataPath, setGameDataPath] = useState<string | null>(null);
  const [gameDataInspection, setGameDataInspection] = useState<GameDataInspection | null>(null);
  const [gameDataError, setGameDataError] = useState<string | null>(null);
  const [botsEnabled, setBotsEnabled] = useState(true);
  const [clientChoice, setClientChoice] = useState<ClientChoice>("managedOpenWow");
  const [aiEnabled, setAiEnabled] = useState(false);
  const [aiCapability, setAiCapability] = useState<AiCapability>(checkingAi);
  const [requestPending, setRequestPending] = useState(false);
  const bootstrapRequest = useRef<Promise<LauncherStatus> | null>(null);
  const aiRequest = useRef<Promise<AiCapability> | null>(null);
  const aiChoiceTouched = useRef(false);
  const installationKnown = useRef(false);

  useEffect(() => {
    let active = true;
    let unlistenProgress: () => void = () => undefined;
    let unlistenStatus: () => void = () => undefined;

    void subscribeLauncherProgress((progress) => {
      if (!active) return;
      setStatus((current) => ({ ...current, ...progress }));
    }).then((stopListening) => { unlistenProgress = stopListening; });
    void subscribeLauncherStatus((next) => {
      if (!active) return;
      setStatus(next);
      setBotsEnabled(next.botsEnabled);
      setAiEnabled(next.aiEnabled);
      setClientChoice(next.clientChoice);
    }).then((stopListening) => { unlistenStatus = stopListening; });

    bootstrapRequest.current ??= bootstrapLauncher();
    void bootstrapRequest.current
      .then((next) => {
        if (!active) return;
        setStatus(next);
        installationKnown.current = next.installed;
        setGameDataPath(next.gameDataPath);
        setBotsEnabled(next.botsEnabled);
        setAiEnabled(next.aiEnabled);
        setClientChoice(next.clientChoice);
        if (next.aiModel) {
          setAiCapability({
            ...checkingAi,
            state: "recommended",
            modelName: next.aiModel,
            ollamaModel: next.aiModel,
            detail: "Modèle local déjà installé et réservé à RealmBox.",
          });
        }
      })
      .catch((error: unknown) => {
        if (!active) return;
        setStatus({ ...initialStatus, phase: "error", message: "RealmBox n’a pas pu démarrer", detail: String(error) });
      });

    aiRequest.current ??= inspectAiCapability();
    void aiRequest.current.then((capability) => {
      if (!active) return;
      setAiCapability(capability);
      if (capability.state === "recommended" && !aiChoiceTouched.current && !installationKnown.current) {
        setAiEnabled(true);
      }
    });

    return () => { active = false; unlistenProgress(); unlistenStatus(); };
  }, []);

  const progressLabel = useMemo(() => {
    if (status.phase === "running") return "MONDE EN COURS";
    if (status.phase === "ready") return "INSTALLATION TERMINÉE";
    if (status.phase === "error") return "INTERVENTION REQUISE";
    if (status.phase === "needsGameData") return "PRÊT À INSTALLER";
    return status.message.toUpperCase();
  }, [status]);

  async function selectData() {
    setRequestPending(true);
    setGameDataError(null);
    try {
      const selected = await chooseGameData();
      if (!selected) return;
      const inspection = await inspectGameData(selected);
      setGameDataInspection(inspection);
      setGameDataPath(inspection.path);
    } catch (error) {
      setGameDataInspection(null);
      setGameDataPath(null);
      setGameDataError(String(error));
    } finally {
      setRequestPending(false);
    }
  }

  async function install() {
    if (!gameDataPath) return;
    setRequestPending(true);
    setStatus((current) => ({ ...current, phase: "installing", message: "Préparation de l’installation", detail: null, progress: 1 }));
    try {
      setStatus(await installRealm(gameDataPath, clientChoice, botsEnabled, aiEnabled, aiCapability.ollamaModel));
    } catch (error) {
      setStatus((current) => ({ ...current, phase: "error", message: "L’installation s’est arrêtée", detail: String(error) }));
    } finally {
      setRequestPending(false);
    }
  }

  async function start() {
    setRequestPending(true);
    try {
      setStatus(await startRealm(botsEnabled, aiEnabled));
    } catch (error) {
      setStatus((current) => ({ ...current, phase: "error", message: "Le monde n’a pas démarré", detail: String(error) }));
    } finally {
      setRequestPending(false);
    }
  }

  async function stop() {
    setRequestPending(true);
    try {
      setStatus(await stopRealm());
    } catch (error) {
      setStatus((current) => ({ ...current, phase: "error", message: "Arrêt incomplet", detail: String(error) }));
    } finally {
      setRequestPending(false);
    }
  }

  return (
    <main className="launcher-shell">
      <div className="launcher-frame">
        <header className="launcher-titlebar">
          <div className="realm-mark" aria-hidden="true"><span>R</span></div>
          <div className="wordmark"><strong>REALMBOX</strong><small>UN MONDE LOCAL · 3.3.5a</small></div>
          <span className="runtime-badge">EXÉCUTION LOCALE</span>
        </header>

        <section className="launcher-stage">
          <aside className="chronicle">
            <div className="realm-crest" aria-hidden="true"><span>R</span><small>REALMBOX</small></div>
            <div className="hero-caption">
              <p className="section-kicker">CHRONIQUES DU NORD</p>
              <h1>Votre royaume.<br/>Votre aventure.</h1>
              <p>Un monde 3.3.5a solitaire, peuplé de compagnons, entièrement sur votre ordinateur.</p>
            </div>
          </aside>

          <section className="launcher-panel" aria-live="polite">
            <div className="panel-cap"><span>NOUVELLES DU ROYAUME</span><span className={`phase-light ${status.phase}`}/></div>

            <div className="status-copy">
              <p className="section-kicker">{progressLabel}</p>
              <h2>{status.message}</h2>
              {status.detail && <p className="status-detail">{status.detail}</p>}
            </div>

            {status.phase === "needsGameData" && <div className="setup-card">
              <fieldset className="client-choice">
                <legend>Client de jeu</legend>
                <label>
                  <input type="radio" name="client" value="managedOpenWow" checked={clientChoice === "managedOpenWow"} onChange={() => setClientChoice("managedOpenWow")}/>
                  <span><strong>OpenWoW géré par RealmBox</strong><small>Recommandé · téléchargé automatiquement et vérifié par SHA-256.</small></span>
                </label>
                <label className={!status.originalClientSupported ? "unavailable" : ""}>
                  <input type="radio" name="client" value="originalWindows" checked={clientChoice === "originalWindows"} disabled={!status.originalClientSupported} onChange={() => setClientChoice("originalWindows")}/>
                  <span><strong>Mon client original</strong><small>{status.originalClientSupported ? "Windows avancé · Wow.exe reste dans votre dossier ; la configuration de royaume est sauvegardée." : "Disponible uniquement dans la version Windows x64."}</small></span>
                </label>
              </fieldset>
              <label>Données de jeu 3.3.5a</label>
              <button className="path-picker" onClick={selectData} disabled={requestPending}>
                <span>{gameDataPath ?? "Choisir le dossier qui contient Data"}</span><b>PARCOURIR</b>
              </button>
              <p className={`data-check ${gameDataError ? "error" : gameDataInspection ? "valid" : ""}`}>
                {gameDataError ?? gameDataInspection?.detail ?? "Requis : Data complet de WoW 3.3.5a build 12340 avec common.MPQ, expansion.MPQ, lichking.MPQ et une locale complète."}
              </p>
              <label className="bot-toggle">
                <input type="checkbox" checked={botsEnabled} onChange={(event) => {
                  setBotsEnabled(event.target.checked);
                  if (!event.target.checked) {
                    aiChoiceTouched.current = true;
                    setAiEnabled(false);
                  }
                }}/>
                <span><strong>Peupler le monde avec des compagnons</strong><small>Active Playerbots au démarrage. Modifiable plus tard.</small></span>
              </label>
              <label className={`bot-toggle ai-toggle ${aiCapability.state !== "recommended" ? "unavailable" : ""}`}>
                <input
                  type="checkbox"
                  checked={aiEnabled}
                  disabled={!botsEnabled || aiCapability.state !== "recommended"}
                  onChange={(event) => {
                    aiChoiceTouched.current = true;
                    setAiEnabled(event.target.checked);
                  }}
                />
                <span>
                  <strong>Dialogues IA 100 % locaux</strong>
                  <small>{aiCapability.state === "checking" ? "Analyse de ce Mac par CanIRun…" : aiCapability.state === "recommended" ? `${aiCapability.modelName} · note ${aiCapability.grade} · env. ${aiCapability.estimatedTokensPerSecond} tokens/s` : "Aucun modèle confortable recommandé sur cette machine."}</small>
                </span>
              </label>
              <p className="hardware-note">{aiCapability.detail} Seuls le processeur, le nombre de cœurs et la mémoire sont envoyés à CanIRun.</p>
              <p className="legal-note">RealmBox ne télécharge aucune donnée propriétaire. Une copie compatible obtenue légalement est nécessaire.</p>
            </div>}

            {status.components.length > 0 && <div className="component-list">
              {status.components.map((rawComponent) => {
                const component = setupComponent(rawComponent, status, botsEnabled, aiEnabled, aiCapability, clientChoice);
                return <div className="component" key={component.id}>
                  <span className={`component-rune ${component.state}`} aria-hidden="true"/>
                  <div><strong>{component.label}</strong><small>{component.detail}</small></div>
                  <em>{component.state === "running" ? "ACTIF" : component.state === "ready" ? "PRÊT" : component.state === "error" ? "ERREUR" : component.state === "missing" && ["bots", "ai"].includes(component.id) ? "CHOISI" : "—"}</em>
                </div>;
              })}
            </div>}

            {status.installed && status.accountName && status.accountPassword && <div className="account-card">
              <span>COMPTE LOCAL</span>
              <strong>{status.accountName} / {status.accountPassword}</strong>
              <small>À saisir dans le client. Ce compte n’est accessible que sur le serveur local.</small>
            </div>}

            {status.phase === "ready" && <div className="ready-toggles">
              <label><input type="checkbox" checked={botsEnabled} onChange={(event) => {
                setBotsEnabled(event.target.checked);
                if (!event.target.checked) {
                  aiChoiceTouched.current = true;
                  setAiEnabled(false);
                }
              }}/><span>Compagnons au prochain démarrage</span></label>
              {status.aiModel && <label><input type="checkbox" checked={aiEnabled} disabled={!botsEnabled} onChange={(event) => {
                aiChoiceTouched.current = true;
                setAiEnabled(event.target.checked);
              }}/><span>Dialogues locaux · {status.aiModel}</span></label>}
            </div>}

            {status.phase === "error" && <div className="error-actions">
              <button onClick={() => void bootstrapLauncher().then(setStatus)}>REVÉRIFIER</button>
              <small>Le diagnostic complet reste local et aucune étape n’est déclarée réussie sans preuve.</small>
            </div>}
          </section>
        </section>

        <footer className="launcher-footer">
          <div className="footer-edition"><strong>3.3.5a</strong><span>ROYAUME LOCAL</span></div>
          <div className="patch-status"><div className="patch-track"><span style={{ width: `${status.progress}%` }}/></div><small>{progressLabel} · {status.progress}%</small></div>
          {status.phase === "needsGameData" && <button className="launch-button" onClick={install} disabled={!gameDataPath || requestPending}>INSTALLER</button>}
          {status.phase === "ready" && <button className="launch-button" onClick={start} disabled={requestPending}>JOUER</button>}
          {status.phase === "running" && <button className="launch-button stop" onClick={stop} disabled={requestPending}>ARRÊTER</button>}
          {isBusy(status) && <button className="launch-button" disabled>VEUILLEZ PATIENTER</button>}
        </footer>
      </div>
      <p className="launcher-version">RealmBox 0.1.0 · {status.platformLabel} · serveur local uniquement</p>
    </main>
  );
}
