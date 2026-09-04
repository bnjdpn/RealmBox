import { useEffect, useRef, useState, type ReactNode } from "react";
import { messages, type Language } from "./i18n";
import { setupMessages } from "./setupCopy";
import { inspectInstallation, openSetupResource } from "./runtime";
import type { AiCapability, BotPresence, ClientChoice, GameDataInspection, InstallationCheck, SetupResource } from "./types";

interface Props {
  language: Language; gameDataPath: string | null; inspection: GameDataInspection | null;
  gameDataError: string | null; busy: boolean; selectData: () => Promise<void>; install: () => Promise<void>; openDiagnostics: () => void;
  clientChoice: ClientChoice; setClientChoice: (choice: ClientChoice) => void; originalClientSupported: boolean;
  worldControls: ReactNode; botsEnabled: boolean; botCount: number; botPresence: BotPresence;
  aiEnabled: boolean; setAiEnabled: (enabled: boolean) => void; capability: AiCapability;
}

function installationReady(check: InstallationCheck | null): boolean {
  return !!check && check.freshTarget && check.platformSupported && check.dockerReady && check.composeReady
    && check.availableBytes !== null && check.availableBytes >= check.requiredBytes;
}

export default function SetupWizard(props: Props) {
  const { language, gameDataPath, inspection, busy, clientChoice, botsEnabled, botCount, botPresence, aiEnabled, capability } = props;
  const copy = messages[language];
  const text = setupMessages[language];
  const [step, setStep] = useState(0);
  const [check, setCheck] = useState<InstallationCheck | null>(null);
  const [checking, setChecking] = useState(false);
  const [checkFailed, setCheckFailed] = useState(false);
  const [helpFailed, setHelpFailed] = useState(false);
  const request = useRef(0);
  const title = useRef<HTMLHeadingElement>(null);
  const model = aiEnabled ? capability.ollamaModel : null;
  const hasData = !!gameDataPath && inspection?.path === gameDataPath;
  const aiReady = !aiEnabled || (botsEnabled && capability.state === "recommended" && !!model && capability.diskSpaceSufficient !== false);
  // A changed optional model must invalidate the disk check, including an in-flight result.
  useEffect(() => { request.current += 1; setCheck(null); setChecking(false); setCheckFailed(false); }, [model]);
  useEffect(() => () => { request.current += 1; }, []);
  useEffect(() => { title.current?.focus(); }, [step]);

  async function verify() {
    if (checking || busy) return;
    const id = ++request.current;
    setChecking(true); setCheck(null); setCheckFailed(false);
    try { const result = await inspectInstallation(model); if (id === request.current) setCheck(result); }
    catch { if (id === request.current) setCheckFailed(true); }
    finally { if (id === request.current) setChecking(false); }
  }
  function go(next: number) {
    if (busy || (next > 0 && !hasData)) return;
    setStep(next);
    if (next === 2 && !checking) void verify();
  }
  async function help(resource: SetupResource) {
    setHelpFailed(false);
    try { await openSetupResource(resource); } catch { setHelpFailed(true); }
  }
  const size = (bytes: number) => `${new Intl.NumberFormat(language, { maximumFractionDigits: 1 }).format(bytes / 1024 ** 3)} ${language === "fr" ? "Gio" : "GiB"}`;
  const presence = { dispersed: copy.presenceDispersed, natural: copy.presenceNatural, close: copy.presenceClose }[botPresence];
  const steps = [text.game, text.world, text.review];

  return <section className="setup-wizard" aria-label={text.steps}>
    <aside className="setup-sidebar">
      <p className="setup-eyebrow">{text.eyebrow}</p>
      <h1>{copy.installTitle}</h1>
      <nav aria-label={text.steps}><ol>{steps.map((label, index) => <li key={label}>
        <button aria-current={step === index ? "step" : undefined} disabled={busy || (index > 0 && !hasData)} onClick={() => go(index)}><span aria-hidden="true">{index + 1}</span>{label}</button>
      </li>)}</ol></nav>
      <p className="helper">{text.noDownload}</p>
    </aside>
    <div className="setup-main">
      <header><h2 ref={title} tabIndex={-1}>{steps[step]}</h2><p>{[text.gameBody, text.worldBody, text.reviewBody][step]}</p></header>
      <div className="setup-scroll" key={step}>
        {step === 0 && <>
          <div className={`setup-folder ${hasData ? "recognized" : ""}`}>
            <strong>{hasData ? `${copy.dataReady} · ${inspection?.locale}` : copy.gameData}</strong>
            {hasData && <><code title={gameDataPath ?? undefined}>{gameDataPath}</code><p className="helper">{text.buildPending}</p></>}
            <button className="secondary-action full" onClick={props.selectData} disabled={busy}>{hasData ? copy.changeFolder : copy.chooseData}</button>
            <p className="helper">{copy.dataRequirement}</p>
          </div>
          {props.gameDataError && <div role="alert"><p className="error-message">{text.selectionFailed}</p><button className="text-button" onClick={props.openDiagnostics}>{copy.openDiagnostics}</button></div>}
          <details className="setup-help"><summary>{text.gameHelp}</summary><p>{text.gameHelpBody}</p><p>{text.macHelp}</p><button className="text-button" onClick={() => void help(language === "fr" ? "gameFr" : "gameEn")}>{text.gameDownload} ↗</button></details>
          <fieldset className="choice-group" disabled={busy}><legend>{copy.gameClient}</legend>
            <label className="choice-card"><input type="radio" name="setup-client" checked={clientChoice === "managedOpenWow"} onChange={() => props.setClientChoice("managedOpenWow")} /><span><strong>{copy.managedClient}</strong><small>{copy.managedClientHelp}</small></span></label>
            {props.originalClientSupported && <label className="choice-card"><input type="radio" name="setup-client" checked={clientChoice === "originalWindows"} onChange={() => props.setClientChoice("originalWindows")} /><span><strong>{copy.originalClient}</strong><small>{copy.originalClientHelp}</small></span></label>}
          </fieldset>
        </>}
        {step === 1 && <>
          <fieldset className="setup-controls" disabled={busy}>{props.worldControls}
            <div className="setup-ai"><label className="option-row"><input type="checkbox" checked={aiEnabled} disabled={!botsEnabled || capability.state !== "recommended" || !capability.ollamaModel || capability.diskSpaceSufficient === false} onChange={(event) => props.setAiEnabled(event.target.checked)} /><span><strong>{copy.ai}</strong><small>{text.optionalAi}</small></span></label>
              <p className="helper">{capability.state === "checking" ? copy.aiChecking : capability.state !== "recommended" ? copy.aiUnavailable : `${capability.modelName} · ${capability.downloadSizeGb ?? "?"} GB`}</p>
              {capability.diskSpaceSufficient === false && <p className="error-message">{copy.diskInsufficient}</p>}
              {aiEnabled && <><p className="helper">{text.localAi}</p><p className="helper">{copy.modelLicense} : {capability.modelLicense ?? "—"}</p></>}
            </div>
          </fieldset>
        </>}
        {step === 2 && <>
          <section className="setup-summary"><h3>{text.automatic}</h3><p>{clientChoice === "originalWindows" ? copy.originalPathHelp : text.automaticBody}</p>
            <code className="setup-summary-path">{gameDataPath} · {inspection?.locale}</code>
            <dl><div><dt>{copy.gameClient}</dt><dd>{clientChoice === "managedOpenWow" ? "OpenWoW" : copy.originalClient}</dd></div>
              <div><dt>{copy.companions}</dt><dd>{botsEnabled ? `${botCount} · ${presence}` : copy.off}</dd></div>
              <div><dt>{copy.ai}</dt><dd>{aiEnabled ? capability.modelName : copy.off}</dd></div></dl>
          </section>
          <section className="setup-checks" aria-label={text.prerequisites} aria-busy={checking}>
            <h3>{text.prerequisites}</h3>
            {checking && <p role="status">{text.checking}</p>}
            {checkFailed && <p className="error-message" role="alert">{text.checkFailed}</p>}
            {check && <>
              <ul>{[
                [text.computer, check.platformSupported, text.platformHelp],
                [text.target, check.freshTarget, text.targetBlocked],
                [text.docker, check.dockerReady && check.composeReady, text.dockerHelp],
                [text.disk, check.availableBytes !== null && check.availableBytes >= check.requiredBytes, `${check.availableBytes === null ? text.unknown : size(check.availableBytes)} · ${size(check.requiredBytes)} ${text.required}`],
              ].map(([label, ok, body]) => <li key={String(label)}><div><strong>{label}</strong><span className={ok ? "success" : "error"}>{ok ? copy.ready : text.attention}</span></div>{(!ok || label === text.disk) && <p>{body}</p>}</li>)}</ul>
              {(!check.dockerReady || !check.composeReady) && <button className="text-button" onClick={() => void help("docker")}>{text.dockerLink} ↗</button>}
              {botsEnabled && <p className="helper">{text.planned} : {check.botCapacity === null ? text.unknown : `${Math.min(botCount, check.botCapacity)} ${text.bots}`} · {text.requested} : {botCount}. {text.limitHelp}</p>}
            </>}
            <button className="text-button" onClick={() => void verify()} disabled={checking || busy}>{text.checkAgain}</button>
          </section>
          <p className="helper">{text.pendingCheck}</p><p className="helper">{text.setupTime}</p>
        </>}
        {helpFailed && <p className="error-message" role="alert">{text.helpFailed}</p>}
      </div>
      <footer className="setup-footer">
        {step > 0 ? <button className="back-button" disabled={busy} onClick={() => go(step - 1)}>{copy.back}</button> : <small>{copy.localOnly}</small>}
        {step === 2 && <span id="setup-result" className={`setup-footer-status ${installationReady(check) ? "success" : ""}`}>{checking ? text.checking : installationReady(check) ? text.readyInstall : text.blockedInstall}</span>}
        <button className="primary-action" disabled={busy || !hasData || (step === 2 && (checking || !installationReady(check) || !aiReady))}
          aria-describedby={step === 2 ? "setup-result" : undefined}
          onClick={() => step === 2 ? void props.install() : go(step + 1)}>{busy ? copy.wait : step === 2 ? copy.install : step === 1 ? text.reviewAction : text.next}</button>
      </footer>
    </div>
  </section>;
}
