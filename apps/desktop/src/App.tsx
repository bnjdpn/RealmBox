import { useEffect, useMemo, useRef, useState } from "react";
import {
  bootstrapLauncher,
  chooseGameData,
  installRealm,
  startRealm,
  stopRealm,
  subscribeLauncherProgress,
} from "./runtime";
import type { LauncherStatus } from "./types";

const initialStatus: LauncherStatus = {
  phase: "checking",
  message: "Vérification de l’installation…",
  detail: null,
  progress: 0,
  installed: false,
  botsEnabled: true,
  gameDataPath: null,
  accountName: null,
  accountPassword: null,
  components: [],
};

function isBusy(status: LauncherStatus) {
  return ["checking", "installing", "starting", "stopping"].includes(status.phase);
}

export default function App() {
  const [status, setStatus] = useState(initialStatus);
  const [gameDataPath, setGameDataPath] = useState<string | null>(null);
  const [botsEnabled, setBotsEnabled] = useState(true);
  const [requestPending, setRequestPending] = useState(false);
  const bootstrapRequest = useRef<Promise<LauncherStatus> | null>(null);

  useEffect(() => {
    let active = true;
    let unlisten: () => void = () => undefined;

    void subscribeLauncherProgress((progress) => {
      if (!active) return;
      setStatus((current) => ({ ...current, ...progress }));
    }).then((stopListening) => { unlisten = stopListening; });

    bootstrapRequest.current ??= bootstrapLauncher();
    void bootstrapRequest.current
      .then((next) => {
        if (!active) return;
        setStatus(next);
        setGameDataPath(next.gameDataPath);
        setBotsEnabled(next.botsEnabled);
      })
      .catch((error: unknown) => {
        if (!active) return;
        setStatus({ ...initialStatus, phase: "error", message: "RealmBox n’a pas pu démarrer", detail: String(error) });
      });

    return () => { active = false; unlisten(); };
  }, []);

  const progressLabel = useMemo(() => {
    if (status.phase === "running") return "MONDE EN COURS";
    if (status.phase === "ready") return "INSTALLATION TERMINÉE";
    if (status.phase === "error") return "INTERVENTION REQUISE";
    if (status.phase === "needsGameData") return "PRÊT À INSTALLER";
    return status.message.toUpperCase();
  }, [status]);

  async function selectData() {
    const selected = await chooseGameData();
    if (selected) setGameDataPath(selected);
  }

  async function install() {
    if (!gameDataPath) return;
    setRequestPending(true);
    setStatus((current) => ({ ...current, phase: "installing", message: "Préparation de l’installation", detail: null, progress: 1 }));
    try {
      setStatus(await installRealm(gameDataPath, botsEnabled));
    } catch (error) {
      setStatus((current) => ({ ...current, phase: "error", message: "L’installation s’est arrêtée", detail: String(error) }));
    } finally {
      setRequestPending(false);
    }
  }

  async function start() {
    setRequestPending(true);
    try {
      setStatus(await startRealm(botsEnabled));
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
            <p className="section-kicker">CHRONIQUE DU ROYAUME</p>
            <h1>Votre monde,<br/>sur votre machine.</h1>
            <p className="intro">RealmBox installe un client ouvert, un serveur local et les compagnons que vous choisissez. Les données du jeu restent les vôtres.</p>
            <div className="build-note">
              <span className="build-number">12340</span>
              <div><strong>ÈRE WRATH · 3.3.5a</strong><p>Lanceur autonome pour un monde privé sur votre ordinateur.</p></div>
            </div>
          </aside>

          <section className="launcher-panel" aria-live="polite">
            <div className="panel-cap"><span>ÉTAT DE REALMBOX</span><span className={`phase-light ${status.phase}`}/></div>

            <div className="status-copy">
              <p className="section-kicker">{progressLabel}</p>
              <h2>{status.message}</h2>
              {status.detail && <p className="status-detail">{status.detail}</p>}
            </div>

            {status.phase === "needsGameData" && <div className="setup-card">
              <label>Données de jeu 3.3.5a</label>
              <button className="path-picker" onClick={selectData} disabled={requestPending}>
                <span>{gameDataPath ?? "Choisir le dossier qui contient Data"}</span><b>PARCOURIR</b>
              </button>
              <label className="bot-toggle">
                <input type="checkbox" checked={botsEnabled} onChange={(event) => setBotsEnabled(event.target.checked)}/>
                <span><strong>Peupler le monde avec des compagnons</strong><small>Active Playerbots au démarrage. Modifiable plus tard.</small></span>
              </label>
              <p className="legal-note">RealmBox ne télécharge aucune donnée propriétaire. Une copie compatible obtenue légalement est nécessaire.</p>
            </div>}

            {status.components.length > 0 && <div className="component-list">
              {status.components.map((component) => <div className="component" key={component.id}>
                <span className={`component-rune ${component.state}`} aria-hidden="true"/>
                <div><strong>{component.label}</strong><small>{component.detail}</small></div>
                <em>{component.state === "running" ? "ACTIF" : component.state === "ready" ? "PRÊT" : component.state === "error" ? "ERREUR" : "—"}</em>
              </div>)}
            </div>}

            {status.installed && status.accountName && status.accountPassword && <div className="account-card">
              <span>COMPTE LOCAL</span>
              <strong>{status.accountName} / {status.accountPassword}</strong>
              <small>À saisir dans le client. Ce compte n’est accessible que sur le serveur local.</small>
            </div>}

            {status.phase === "ready" && <label className="ready-bot-toggle">
              <input type="checkbox" checked={botsEnabled} onChange={(event) => setBotsEnabled(event.target.checked)}/>
              <span>Compagnons au prochain démarrage</span>
            </label>}

            {status.phase === "error" && <div className="error-actions">
              <button onClick={() => void bootstrapLauncher().then(setStatus)}>REVÉRIFIER</button>
              <small>Le diagnostic complet reste local et aucune étape n’est déclarée réussie sans preuve.</small>
            </div>}
          </section>
        </section>

        <footer className="launcher-footer">
          <div className="patch-status"><div className="patch-track"><span style={{ width: `${status.progress}%` }}/></div><small>{progressLabel} · {status.progress}%</small></div>
          {status.phase === "needsGameData" && <button className="launch-button" onClick={install} disabled={!gameDataPath || requestPending}>INSTALLER</button>}
          {status.phase === "ready" && <button className="launch-button" onClick={start} disabled={requestPending}>JOUER</button>}
          {status.phase === "running" && <button className="launch-button stop" onClick={stop} disabled={requestPending}>ARRÊTER</button>}
          {isBusy(status) && <button className="launch-button" disabled>VEUILLEZ PATIENTER</button>}
        </footer>
      </div>
      <p className="launcher-version">RealmBox 0.1.0 · OpenWoW 0.1.2 · serveur local uniquement</p>
    </main>
  );
}
