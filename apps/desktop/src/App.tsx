import { useState } from "react";
import oathkeeperPortrait from "./assets/oathkeeper-portrait.webp";
import realmValley from "./assets/realm-valley.webp";
import { copy } from "./i18n";
import { prepareWorld, startWorld, stopWorld, talkToCompanion } from "./runtime";
import { fakeDashboard, type AppView, type Dashboard, type WorldPreset } from "./types";

const presetLabels: Record<WorldPreset, { title: string; description: string }> = {
  calm: { title: "Monde calme", description: "Une présence discrète et une charge minimale." },
  living: { title: "Monde vivant", description: "Des routes animées et des compagnons faciles à trouver." },
  crowded: { title: "Monde très peuplé", description: "Davantage d’habitants pour les machines puissantes." },
};

const roleLabels = { tank: "Tank", healer: "Soins", damage: "Dégâts" } as const;

export default function App() {
  const [view, setView] = useState<AppView>("welcome");
  const [preset, setPreset] = useState<WorldPreset>("living");
  const [dashboard, setDashboard] = useState<Dashboard>(fakeDashboard);
  const [progress, setProgress] = useState(0);
  const [progressMessage, setProgressMessage] = useState("");
  const [chat, setChat] = useState("");
  const [reply, setReply] = useState("");

  async function runPreparation() {
    setView("preparing");
    const result = await prepareWorld((next, message) => { setProgress(next); setProgressMessage(message); });
    setDashboard({ ...result, preset });
    setView("dashboard");
  }

  async function play() {
    setView("starting");
    const result = await startWorld((next, message) => { setProgress(next); setProgressMessage(message); });
    setDashboard({ ...result, preset });
    setView("running");
  }

  async function stop() {
    setDashboard(await stopWorld());
    setView("dashboard");
  }

  async function sendMessage(event: React.FormEvent) {
    event.preventDefault();
    if (!chat.trim()) return;
    setReply(await talkToCompanion("melya", chat));
    setChat("");
  }

  return (
    <main className="shell">
      <header className="topbar"><span className="sigil" aria-hidden="true">R</span><span>{copy.brand}</span><button className="icon-button" aria-label="Ouvrir les paramètres">•••</button></header>

      {view === "welcome" && <section className="hero screen-enter">
        <div className="hero-copy"><p className="eyebrow">{copy.eyebrow}</p><h1>{copy.welcomeTitle}</h1><p className="lede">{copy.welcomeBody}</p><button className="primary" onClick={() => setView("data")}>{copy.prepare}<span aria-hidden="true">→</span></button><p className="fine-print">{copy.legal}</p></div>
        <figure className="world-illustration" aria-hidden="true"><img src={realmValley} alt=""/><span className="image-wash"/><figcaption>UNE FRONTIÈRE À SOI</figcaption></figure>
      </section>}

      {view === "data" && <section className="centered screen-enter"><p className="eyebrow">ÉTAPE 1 SUR 2</p><h2>Trouvons vos données</h2><p className="lede narrow">Elles servent uniquement à faire fonctionner votre monde local et ne quittent jamais cet ordinateur.</p><div className="data-drop"><span className="folder" aria-hidden="true">◇</span><strong>Aucun dossier trouvé automatiquement</strong><span>Choisissez le dossier <code>Data</code> ou son dossier parent.</span><button className="secondary" onClick={() => setView("experience")}>Choisir le dossier de démonstration</button></div><p className="demo-label">PARCOURS FAKE — AUCUN FICHIER RÉEL N’EST LU</p></section>}

      {view === "experience" && <section className="centered screen-enter"><p className="eyebrow">ÉTAPE 2 SUR 2</p><h2>Choisissez votre ambiance</h2><p className="lede narrow">RealmBox ajustera automatiquement la population et l’activité locale.</p><div className="preset-list">{(Object.keys(presetLabels) as WorldPreset[]).map((id) => <button key={id} className={`preset ${preset === id ? "selected" : ""}`} onClick={() => setPreset(id)}><span className="preset-radio"/><span><strong>{presetLabels[id].title}{id === "living" && <em>Recommandé</em>}</strong><small>{presetLabels[id].description}</small></span></button>)}</div><button className="primary" onClick={runPreparation}>Préparer automatiquement<span aria-hidden="true">→</span></button></section>}

      {(view === "preparing" || view === "starting") && <section className="progress-screen screen-enter"><div className="pulse-mark" aria-hidden="true"><span>R</span></div><p className="eyebrow">{view === "preparing" ? "PRÉPARATION INITIALE" : "UN INSTANT"}</p><h2>{progressMessage}</h2><p>{view === "preparing" ? "RealmBox assemble votre monde local." : "Votre aventure reprend là où vous l’avez laissée."}</p><div className="progress-track" role="progressbar" aria-valuemin={0} aria-valuemax={100} aria-valuenow={progress}><span style={{ width: `${progress}%` }}/></div><strong className="progress-number">{progress}%</strong><span className="fake-pill">Simulation locale</span></section>}

      {(view === "dashboard" || view === "running") && <section className="dashboard screen-enter">
        <aside className="character-panel"><div className="avatar" aria-hidden="true"><img src={oathkeeperPortrait} alt=""/></div><p className="eyebrow">BON RETOUR</p><h2>{dashboard.playerName}</h2><p>{dashboard.className} · niveau {dashboard.level}</p><div className="world-status"><span className={dashboard.sessionRunning ? "status-live" : "status-ready"}/><div><strong>{dashboard.sessionRunning ? "Monde éveillé" : "Monde prêt"}</strong><small>{presetLabels[preset].title} · IA locale prête</small></div></div></aside>
        <div className="play-panel"><p className="eyebrow">{dashboard.sessionRunning ? "SESSION EN COURS" : "PRÊT POUR L’AVENTURE"}</p><h1>{dashboard.sessionRunning ? "Vos compagnons vous attendent." : "Le monde vous attend."}</h1><p>{dashboard.evidence}</p>{dashboard.sessionRunning ? <button className="stop-button" onClick={stop}>Fermer la session proprement</button> : <button className="play-button" onClick={play}><span>JOUER</span><small>Un clic, et votre monde s’éveille</small></button>}</div>
        <section className="companions"><div className="section-title"><div><p className="eyebrow">VOTRE GROUPE</p><h3>Compagnons habituels</h3></div><button className="text-button">Gérer</button></div><div className="companion-list">{dashboard.companions.map((companion) => <article key={companion.id}><div className={`portrait ${companion.role}`}>{companion.name.slice(0, 1)}</div><div><strong>{companion.name}</strong><small>{roleLabels[companion.role]} · niveau {companion.level}</small></div><span className="ready-dot" aria-label="Prêt"/></article>)}</div>{dashboard.sessionRunning && <form className="chat" onSubmit={sendMessage}><label htmlFor="companion-chat">Parler à Melya</label><div><input id="companion-chat" value={chat} onChange={(event) => setChat(event.target.value)} placeholder="On est prêts pour la suite ?"/><button type="submit" aria-label="Envoyer">→</button></div>{reply && <output>{reply}</output>}</form>}</section>
      </section>}
    </main>
  );
}
