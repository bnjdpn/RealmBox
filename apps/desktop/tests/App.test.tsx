import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import axe from "axe-core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "../src/App";
import type { InstallationCheck, LauncherStatus, SoloProfileView } from "../src/types";

const missing: LauncherStatus = {
  phase: "needsGameData",
  message: "Données de jeu requises",
  detail: null,
  errorCode: null,
  progress: 0,
  installed: false,
  recoveryAvailable: false,
  botsEnabled: true,
  botCount: 50,
  requestedBotCount: 50,
  appliedBotCount: 50,
  botPresence: "natural",
  aiEnabled: false,
  aiModel: null,
  dialogueChattiness: "balanced",
  gameDataPath: null,
  accountName: null,
  accountPassword: null,
  clientChoice: "managedOpenWow",
  originalClientSupported: true,
  platformLabel: "Windows x64",
  components: [
    { id: "client", label: "Client de jeu", state: "missing", detail: "À préparer" },
    { id: "database", label: "Sauvegarde du royaume", state: "missing", detail: "À préparer" },
    { id: "server", label: "Monde privé", state: "missing", detail: "À préparer" },
    { id: "bots", label: "Compagnons", state: "missing", detail: "Optionnels" },
    { id: "ai", label: "Dialogues vivants", state: "stopped", detail: "Selon cette machine" },
  ],
};

const ready: LauncherStatus = {
  ...missing,
  phase: "ready",
  message: "Installation terminée",
  progress: 100,
  installed: true,
  gameDataPath: "/Jeux/Wrath",
  accountName: "REALMBOX",
  accountPassword: "REALMBOX",
  components: missing.components.map((component) => ({ ...component, state: "ready", detail: "Installé" })),
};

const running: LauncherStatus = {
  ...ready,
  phase: "running",
  message: "Le monde est lancé",
  components: ready.components.map((component) => ({ ...component, state: "running", detail: "Actif" })),
};

const soloView: SoloProfileView = {
  activeProfile: "normal", rollbackAvailable: false, pendingChange: false,
  profiles: (["normal", "comfortable", "accelerated"] as const).map((profile, index) => ({
    profile, catalogVersion: 1,
    labelFr: ["Normal", "Confort", "Accéléré"][index],
    labelEn: ["Normal", "Comfortable", "Accelerated"][index],
    settings: [
      { key: "Rate.XP.Kill", value: String(index + 1) },
      { key: "Rate.Reputation.Gain", value: String(index + 1) },
      { key: "Rate.Drop.Money", value: index === 2 ? "2" : "1" },
      { key: "MaxPrimaryTradeSkill", value: index ? "11" : "2" },
      { key: "Instance.IgnoreRaid", value: index ? "1" : "0" },
      { key: "Instance.IgnoreLevel", value: index ? "1" : "0" },
      { key: "Quests.IgnoreRaid", value: index ? "1" : "0" },
    ],
  })),
};

const runtime = vi.hoisted(() => ({
  bootstrapLauncher: vi.fn(),
  changeGameDataPath: vi.fn(),
  chooseGameData: vi.fn(),
  configureLocalDialogue: vi.fn(),
  configureDialogueChattiness: vi.fn(),
  createRealmBackup: vi.fn(),
  installRealm: vi.fn(),
  restoreLastRecovery: vi.fn(),
  startRealm: vi.fn(),
  stopRealm: vi.fn(),
  subscribeLauncherProgress: vi.fn(),
  subscribeLauncherStatus: vi.fn(),
  inspectAiCapability: vi.fn(),
  inspectGameData: vi.fn(),
  inspectInstallation: vi.fn(),
  openSetupResource: vi.fn(),
  inspectRealmBackup: vi.fn(),
  queryLocalGuide: vi.fn(),
  inspectSoloProfiles: vi.fn(),
  configureSoloProfile: vi.fn(),
  rollbackSoloProfile: vi.fn(),
  updatePlayerbotPopulation: vi.fn(),
  getRealmDiagnostics: vi.fn(),
}));

vi.mock("../src/runtime", () => runtime);

describe("RealmBox launcher", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    localStorage.setItem("realmbox-language", "fr");
    runtime.bootstrapLauncher.mockResolvedValue(missing);
    runtime.changeGameDataPath.mockResolvedValue(ready);
    runtime.chooseGameData.mockResolvedValue("/Jeux/Wrath");
    runtime.inspectGameData.mockResolvedValue({
      path: "/Jeux/Wrath",
      locale: "frFR",
      detail: "Données WotLK frFR reconnues ; la build 12340 sera confirmée par les extracteurs locaux.",
    });
    runtime.installRealm.mockResolvedValue(ready);
    runtime.inspectInstallation.mockResolvedValue({ freshTarget: true, platformSupported: true, dockerReady: true, composeReady: true, availableBytes: 80 * 1024 ** 3, requiredBytes: 24 * 1024 ** 3, botCapacity: 50 });
    runtime.openSetupResource.mockResolvedValue(undefined);
    runtime.restoreLastRecovery.mockResolvedValue(ready);
    runtime.startRealm.mockResolvedValue(running);
    runtime.stopRealm.mockResolvedValue(ready);
    runtime.configureLocalDialogue.mockResolvedValue({ ...ready, aiEnabled: true, aiModel: "llama3.2:3b" });
    runtime.configureDialogueChattiness.mockResolvedValue({ ...ready, aiEnabled: true, aiModel: "llama3.2:3b", dialogueChattiness: "lively" });
    runtime.inspectRealmBackup.mockResolvedValue({ createdAtUnixMs: 1_788_437_400_000, sizeBytes: 4_194_304 });
    runtime.createRealmBackup.mockResolvedValue({ createdAtUnixMs: 1_788_437_460_000, sizeBytes: 4_325_376 });
    runtime.queryLocalGuide.mockResolvedValue({ entries: [], provenance: null, uncertainty: "none" });
    runtime.inspectSoloProfiles.mockResolvedValue(soloView);
    runtime.configureSoloProfile.mockResolvedValue({ ...soloView, activeProfile: "comfortable", rollbackAvailable: true });
    runtime.rollbackSoloProfile.mockResolvedValue(soloView);
    runtime.updatePlayerbotPopulation.mockResolvedValue({ ...running, botCount: 100, requestedBotCount: 100, appliedBotCount: 100, botPresence: "close" });
    runtime.getRealmDiagnostics.mockResolvedValue({
      summary: "Aucune erreur récente détectée dans les journaux gérés.",
      component: "launcher",
      logsPath: "/RealmBox/logs",
      recentEntries: [],
    });
    runtime.subscribeLauncherProgress.mockResolvedValue(() => undefined);
    runtime.subscribeLauncherStatus.mockResolvedValue(() => undefined);
    runtime.inspectAiCapability.mockResolvedValue({
      state: "recommended",
      deviceName: "Apple M4 Max",
      ramGb: 36,
      modelId: "llama3.2-3b",
      modelName: "Llama 3.2 3B",
      ollamaModel: "llama3.2:3b",
      grade: "S",
      estimatedTokensPerSecond: 177,
      downloadSizeGb: 2,
      diskAvailableGb: 80,
      diskSpaceSufficient: true,
      modelLicense: "Llama 3.2 Community License",
      detail: "CanIRun le classe confortable.",
      sourceUrl: "https://www.canirun.ai/",
    });
  });

  it("requires owned game data before the real installation", async () => {
    const user = userEvent.setup();
    render(<App />);

    expect(await screen.findByRole("heading", { name: /préparer mon monde/i })).toBeVisible();
    await user.click(screen.getByRole("button", { name: /choisir le dossier/i }));
    expect(runtime.inspectGameData).toHaveBeenCalledWith("/Jeux/Wrath");
    expect(screen.getByText("/Jeux/Wrath")).toBeVisible();
    expect(screen.getByText(/archives reconnues/i)).toBeVisible();
    expect(screen.queryByRole("button", { name: /^installer$/i })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /^continuer$/i }));
    await screen.findByText(/Llama 3\.2 3B/i);
    await user.click(screen.getByRole("checkbox", { name: /dialogues locaux/i }));
    await user.click(screen.getByRole("button", { name: /vérifier mon installation/i }));
    expect(await screen.findByRole("button", { name: /^installer$/i })).toBeEnabled();
    await user.click(screen.getByRole("button", { name: /^installer$/i }));
    expect(runtime.installRealm).toHaveBeenCalledWith("/Jeux/Wrath", "managedOpenWow", true, 50, "natural", true, "llama3.2:3b");
    expect(await screen.findByRole("button", { name: /jouer/i })).toBeVisible();
  });

  async function reachReview(user: ReturnType<typeof userEvent.setup>) {
    await user.click(await screen.findByRole("button", { name: /choisir le dossier/i }));
    await user.click(screen.getByRole("button", { name: /^continuer$/i }));
    await user.click(screen.getByRole("button", { name: /vérifier mon installation/i }));
  }

  it("gates setup steps on inspection and preserves the selected folder when the picker is cancelled", async () => {
    const user = userEvent.setup();
    render(<App />);
    expect(await screen.findByRole("button", { name: /^continuer$/i })).toBeDisabled();
    expect(screen.getByRole("button", { name: /votre installation/i })).toBeDisabled();
    await user.click(screen.getByRole("button", { name: /choisir le dossier/i }));
    runtime.chooseGameData.mockResolvedValueOnce(null);
    await user.click(screen.getByRole("button", { name: /changer de dossier/i }));
    expect(screen.getByText("/Jeux/Wrath")).toBeVisible();
    expect(screen.getByRole("button", { name: /^continuer$/i })).toBeEnabled();
    expect(runtime.installRealm).not.toHaveBeenCalled();
  });

  it.each([
    ["Docker", { dockerReady: false }], ["Compose", { composeReady: false }],
    ["platform", { platformSupported: false }], ["existing realm", { freshTarget: false }],
    ["unknown disk", { availableBytes: null }], ["insufficient disk", { availableBytes: 1 }],
  ])("blocks installation when the %s check fails", async (_, failure) => {
    const user = userEvent.setup();
    const baseline = await runtime.inspectInstallation();
    runtime.inspectInstallation.mockResolvedValue({ ...baseline, ...failure });
    render(<App />);
    await reachReview(user);
    expect(screen.getByRole("button", { name: /^installer$/i })).toBeDisabled();
    expect(runtime.installRealm).not.toHaveBeenCalled();
  });

  it("keeps install disabled during the check and offers a retry after a check error", async () => {
    const user = userEvent.setup();
    let rejectCheck: (reason: unknown) => void = () => undefined;
    runtime.inspectInstallation.mockImplementationOnce(() => new Promise((_, reject) => { rejectCheck = reject; }));
    render(<App />);
    await reachReview(user);
    expect(screen.getByRole("button", { name: /^installer$/i })).toBeDisabled();
    expect(screen.getAllByText(/vérification de cet ordinateur/i)[0]).toBeVisible();
    await user.click(screen.getByRole("button", { name: /votre installation/i }));
    expect(runtime.inspectInstallation).toHaveBeenCalledTimes(1);
    await act(async () => rejectCheck(new Error("probe failed")));
    expect(screen.getByRole("alert")).toHaveTextContent(/rien n’a été installé/i);
    await user.click(screen.getByRole("button", { name: /vérifier à nouveau/i }));
    expect(screen.getByRole("button", { name: /^installer$/i })).toBeEnabled();
  });

  it("preserves independent choices across back navigation and never silently raises the detected bot limit", async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(await screen.findByRole("button", { name: /choisir le dossier/i }));
    await user.click(screen.getByRole("button", { name: /^continuer$/i }));
    await user.click(screen.getByRole("radio", { name: /monde dense/i }));
    await user.click(screen.getByRole("radio", { name: /dispersés/i }));
    await user.click(screen.getByRole("button", { name: /vérifier mon installation/i }));
    expect(screen.getByText(/prévu avec cette mémoire : 50 bots/i)).toBeVisible();
    await user.click(screen.getByRole("button", { name: /^retour$/i }));
    expect(screen.getByRole("radio", { name: /monde dense/i })).toBeChecked();
    expect(screen.getByRole("radio", { name: /dispersés/i })).toBeChecked();
    await user.click(screen.getByRole("button", { name: /vérifier mon installation/i }));
    await user.click(screen.getByRole("button", { name: /^installer$/i }));
    expect(runtime.installRealm).toHaveBeenCalledWith("/Jeux/Wrath", "managedOpenWow", true, 100, "dispersed", false, null);
  });

  it("invalidates a stale disk check when the optional model changes", async () => {
    const user = userEvent.setup();
    let resolveOld: (value: InstallationCheck) => void = () => undefined;
    const baseline = await runtime.inspectInstallation();
    runtime.inspectInstallation.mockImplementationOnce(() => new Promise((resolve) => { resolveOld = resolve; }));
    render(<App />);
    await reachReview(user);
    await user.click(screen.getByRole("button", { name: /^retour$/i }));
    await user.click(screen.getByRole("checkbox", { name: /dialogues locaux/i }));
    runtime.inspectInstallation.mockResolvedValue({ ...baseline, availableBytes: 24 * 1024 ** 3, requiredBytes: 26 * 1024 ** 3 });
    await user.click(screen.getByRole("button", { name: /vérifier mon installation/i }));
    await act(async () => resolveOld(baseline));
    expect(runtime.inspectInstallation).toHaveBeenLastCalledWith("llama3.2:3b");
    expect(screen.getByRole("button", { name: /^installer$/i })).toBeDisabled();
  });

  it("keeps the setup draft after install failure and retry, without automatically installing again", async () => {
    const user = userEvent.setup();
    runtime.installRealm.mockRejectedValueOnce({ code: "downloadInterrupted" });
    render(<App />);
    await reachReview(user);
    await user.click(screen.getByRole("button", { name: /^retour$/i }));
    await user.click(screen.getByRole("radio", { name: /aventure tranquille/i }));
    await user.click(screen.getByRole("button", { name: /vérifier mon installation/i }));
    await user.click(screen.getByRole("button", { name: /^installer$/i }));
    await user.click(await screen.findByRole("button", { name: /réessayer/i }));
    expect(screen.getByText("/Jeux/Wrath")).toBeVisible();
    await user.click(screen.getByRole("button", { name: /^continuer$/i }));
    expect(screen.getByRole("radio", { name: /aventure tranquille/i })).toBeChecked();
    expect(runtime.installRealm).toHaveBeenCalledTimes(1);
  });

  it("opens only the chosen language help page and leaves the game selection untouched", async () => {
    const user = userEvent.setup();
    render(<App />);
    await user.click(await screen.findByText(/je n’ai pas encore les fichiers/i));
    await user.click(screen.getByRole("button", { name: /chromiecraft/i }));
    expect(runtime.openSetupResource).toHaveBeenCalledWith("gameFr");
    expect(runtime.chooseGameData).not.toHaveBeenCalled();
    expect(runtime.installRealm).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: /^continuer$/i })).toBeDisabled();
  });

  it("has no automated accessibility violation across the three setup steps", async () => {
    const user = userEvent.setup();
    const { container } = render(<App />);
    const options = { rules: { "color-contrast": { enabled: false } } };
    await screen.findByRole("button", { name: /^continuer$/i });
    expect((await axe.run(container, options)).violations).toEqual([]);
    await user.click(screen.getByRole("button", { name: /choisir le dossier/i }));
    await user.click(screen.getByRole("button", { name: /^continuer$/i }));
    expect((await axe.run(container, options)).violations).toEqual([]);
    await user.click(screen.getByRole("button", { name: /vérifier mon installation/i }));
    await waitFor(() => expect(screen.getByRole("button", { name: /^installer$/i })).toBeEnabled());
    expect((await axe.run(container, options)).violations).toEqual([]);
  });

  it("reviews and installs in English with explicit model consent and localized download help", async () => {
    const user = userEvent.setup();
    localStorage.setItem("realmbox-language", "en");
    render(<App />);
    await user.click(await screen.findByText(/I don’t have the WoW files yet/i));
    await user.click(screen.getByRole("button", { name: /ChromieCraft downloads/i }));
    expect(runtime.openSetupResource).toHaveBeenCalledWith("gameEn");
    await user.click(screen.getByRole("button", { name: /choose the folder/i }));
    expect(runtime.chooseGameData).toHaveBeenCalledWith("en");
    await user.click(screen.getByRole("button", { name: /^continue$/i }));
    expect(screen.getByRole("checkbox", { name: /local dialogue/i })).not.toBeChecked();
    await user.click(screen.getByRole("checkbox", { name: /local dialogue/i }));
    expect(screen.getByText(/No paid dialogue service is activated/i)).toBeVisible();
    await user.click(screen.getByRole("button", { name: /check my installation/i }));
    expect(screen.getByText(/24 GiB required/i)).toBeVisible();
    expect(screen.getByText(/Planned with this memory/i)).toBeVisible();
    await user.click(screen.getByRole("button", { name: /^install$/i }));
    expect(await screen.findByRole("button", { name: /^play$/i })).toBeVisible();
    expect(runtime.installRealm).toHaveBeenCalledWith("/Jeux/Wrath", "managedOpenWow", true, 50, "natural", true, "llama3.2:3b");
  });

  it("rejects incomplete game data before installation starts", async () => {
    const user = userEvent.setup();
    runtime.inspectGameData.mockRejectedValue("archive WotLK requise absente : Data/lichking.MPQ");
    render(<App />);

    await screen.findByRole("heading", { name: /préparer mon monde/i });
    await user.click(screen.getByRole("button", { name: /choisir le dossier/i }));

    expect(await screen.findByRole("alert")).toHaveTextContent(/ce dossier n’a pas pu être utilisé/i);
    expect(screen.queryByRole("button", { name: /^installer$/i })).not.toBeInTheDocument();
    expect(runtime.installRealm).not.toHaveBeenCalled();
    await user.click(screen.getByRole("button", { name: /voir le diagnostic/i }));
    await user.click(await screen.findByText(/^cause$/i));
    expect(screen.getByText(/lichking\.MPQ/i)).toBeVisible();
  });

  it("surfaces a native folder-picker failure instead of leaving the button inert", async () => {
    const user = userEvent.setup();
    runtime.chooseGameData.mockRejectedValue("dialog.open not allowed");
    render(<App />);

    await screen.findByRole("heading", { name: /préparer mon monde/i });
    await user.click(screen.getByRole("button", { name: /choisir le dossier/i }));

    expect(await screen.findByRole("alert")).toHaveTextContent(/ce dossier n’a pas pu être utilisé/i);
    expect(screen.queryByText(/dialog\.open not allowed/i)).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /^installer$/i })).not.toBeInTheDocument();
    expect(runtime.inspectGameData).not.toHaveBeenCalled();
  });

  it("persists the player-provided Windows client choice", async () => {
    const user = userEvent.setup();
    render(<App />);

    await screen.findByRole("heading", { name: /préparer mon monde/i });
    await user.click(screen.getByRole("radio", { name: /mon client original/i }));
    await user.click(screen.getByRole("button", { name: /choisir le dossier/i }));
    await user.click(screen.getByRole("button", { name: /^continuer$/i }));
    await user.click(screen.getByRole("button", { name: /vérifier mon installation/i }));
    await user.click(screen.getByRole("button", { name: /^installer$/i }));

    expect(runtime.installRealm).toHaveBeenCalledWith(
      "/Jeux/Wrath",
      "originalWindows",
      true,
      50,
      "natural",
      false,
      null,
    );
  });

  it("keeps local dialogue disabled when CanIRun finds no comfortable model", async () => {
    runtime.inspectAiCapability.mockResolvedValue({
      state: "unavailable",
      deviceName: "Apple M1",
      ramGb: 8,
      modelId: null,
      modelName: null,
      ollamaModel: null,
      grade: null,
      estimatedTokensPerSecond: null,
      downloadSizeGb: null,
      modelLicense: null,
      detail: "Mémoire insuffisante.",
      sourceUrl: "https://www.canirun.ai/",
    });
    render(<App />);

    const user = userEvent.setup();
    await user.click(await screen.findByRole("button", { name: /choisir le dossier/i }));
    await user.click(screen.getByRole("button", { name: /^continuer$/i }));
    expect(await screen.findByText(/aucun petit modèle confortable/i)).toBeVisible();
    expect(screen.getByRole("checkbox", { name: /dialogues locaux/i })).toBeDisabled();
  });

  it("renders the already-started result returned on a later launch", async () => {
    runtime.bootstrapLauncher.mockResolvedValue(running);
    render(<App />);

    expect(await screen.findByRole("heading", { name: /votre monde est ouvert/i })).toBeVisible();
    expect(screen.getByRole("button", { name: /arrêter le monde/i })).toBeVisible();
    expect(screen.queryByRole("button", { name: /installer/i })).not.toBeInTheDocument();
  });

  it("announces a detected Docker purge in French and English before rebuilding", async () => {
    const user = userEvent.setup();
    runtime.bootstrapLauncher.mockResolvedValue({
      ...ready,
      message: "Les ressources Docker seront reconstruites depuis la sauvegarde locale vérifiée au prochain lancement",
    });
    render(<App />);

    expect(await screen.findByText(/ressources Docker seront reconstruites/i)).toBeVisible();
    expect(screen.getByRole("button", { name: /jouer/i })).toBeVisible();

    await user.click(screen.getByRole("button", { name: /réglages/i }));
    await user.click(screen.getByRole("button", { name: "English" }));
    await user.click(screen.getByRole("button", { name: /^close$/i }));
    expect(screen.getByText(/rebuilt from the verified local backup/i)).toBeVisible();
  });

  it("returns to the ready state when the supervised client exits", async () => {
    let publishStatus: ((status: LauncherStatus) => void) | undefined;
    runtime.bootstrapLauncher.mockResolvedValue(running);
    runtime.subscribeLauncherStatus.mockImplementation(async (listener) => {
      publishStatus = listener;
      return () => undefined;
    });
    render(<App />);
    expect(await screen.findByRole("button", { name: /arrêter le monde/i })).toBeVisible();

    act(() => publishStatus?.(ready));
    expect(await screen.findByRole("button", { name: /jouer/i })).toBeVisible();
  });

  it("switches the complete player flow to English", async () => {
    const user = userEvent.setup();
    render(<App />);
    await screen.findByRole("heading", { name: /préparer mon monde/i });

    await user.click(screen.getByRole("button", { name: /réglages/i }));
    await user.click(screen.getByRole("button", { name: "English" }));
    await user.click(screen.getByRole("button", { name: /^close$/i }));
    expect(screen.getByRole("heading", { name: /set up my world/i })).toBeVisible();
    expect(screen.getByText(/does not download proprietary game data/i)).toBeVisible();
    expect(screen.getByRole("button", { name: /choose the folder/i })).toBeVisible();
    await user.click(screen.getByRole("button", { name: /choose the folder/i }));
    await user.click(screen.getByRole("button", { name: /^continue$/i }));
    expect(screen.getByRole("radio", { name: /natural/i })).toBeVisible();
  });

  it("applies a running bot population without stopping the client", async () => {
    const user = userEvent.setup();
    runtime.bootstrapLauncher.mockResolvedValue(running);
    render(<App />);
    await screen.findByRole("button", { name: /arrêter le monde/i });

    await user.click(screen.getByRole("button", { name: /réglages/i }));
    await user.click(screen.getByRole("button", { name: /^compagnons\b/i }));
    expect(screen.getByRole("heading", { name: /compagnons et présence/i })).toBeVisible();
    await user.selectOptions(screen.getByRole("combobox", { name: /profil du monde/i }), "dense");
    await user.click(screen.getByRole("radio", { name: /toujours proches/i }));
    await user.click(screen.getByRole("button", { name: /appliquer maintenant/i }));

    expect(runtime.updatePlayerbotPopulation).toHaveBeenCalledWith(true, 100, "close");
    expect(runtime.stopRealm).not.toHaveBeenCalled();
    expect(await screen.findByText(/sans redémarrer le client/i)).toBeVisible();
  });

  it("saves a dispersed presence for the next game while the world is stopped", async () => {
    const user = userEvent.setup();
    runtime.bootstrapLauncher.mockResolvedValue(ready);
    runtime.updatePlayerbotPopulation.mockResolvedValue({ ...ready, botPresence: "dispersed" });
    render(<App />);
    await screen.findByRole("button", { name: /^jouer$/i });

    await user.click(screen.getByRole("button", { name: /réglages/i }));
    await user.click(screen.getByRole("button", { name: /^compagnons\b/i }));
    await user.click(screen.getByRole("radio", { name: /dispersés/i }));
    await user.click(screen.getByRole("button", { name: /enregistrer pour la prochaine partie/i }));

    expect(runtime.updatePlayerbotPopulation).toHaveBeenCalledWith(true, 50, "dispersed");
    expect(await screen.findByText(/enregistrées pour la prochaine partie/i)).toBeVisible();
  });

  it("creates a complete verified backup from the player-facing protection panel", async () => {
    const user = userEvent.setup();
    runtime.bootstrapLauncher.mockResolvedValue(running);
    render(<App />);
    await screen.findByRole("button", { name: /arrêter le monde/i });

    await user.click(screen.getByRole("button", { name: /réglages/i }));
    await user.click(screen.getByRole("button", { name: /^protection/i }));

    expect(await screen.findByRole("heading", { name: /protection du monde/i })).toBeVisible();
    expect(await screen.findByText(/^vérifiée$/i)).toBeVisible();
    expect(screen.getByText(/le monde reste ouvert/i)).toBeVisible();
    await user.click(screen.getByRole("button", { name: /sauvegarder maintenant/i }));

    expect(runtime.createRealmBackup).toHaveBeenCalledTimes(1);
    expect(await screen.findByText(/nouvelle sauvegarde complète et vérifiée/i)).toBeVisible();
    expect(runtime.stopRealm).not.toHaveBeenCalled();
  });

  it("turns local dialogue off when companions are disabled", async () => {
    const user = userEvent.setup();
    runtime.bootstrapLauncher.mockResolvedValue({ ...ready, aiEnabled: true, aiModel: "llama3.2:3b" });
    runtime.updatePlayerbotPopulation.mockResolvedValue({ ...ready, botsEnabled: false, aiEnabled: false, botCount: 0, appliedBotCount: 0 });
    render(<App />);
    await screen.findByRole("button", { name: /^jouer$/i });

    await user.click(screen.getByRole("button", { name: /réglages/i }));
    await user.click(screen.getByRole("button", { name: /^compagnons\b/i }));
    await user.click(screen.getByRole("checkbox", { name: /peupler le monde/i }));
    await user.click(screen.getByRole("button", { name: /enregistrer pour la prochaine partie/i }));
    await user.click(screen.getByRole("button", { name: /retour/i }));
    await user.click(screen.getByRole("button", { name: /fermer/i }));
    await user.click(screen.getByRole("button", { name: /^jouer$/i }));

    expect(runtime.updatePlayerbotPopulation).toHaveBeenCalledWith(false, 50, "natural");
    expect(runtime.startRealm).toHaveBeenCalledWith(false, 50, "natural", false);
  });

  it("searches local references only after an explicit player request", async () => {
    const user = userEvent.setup();
    const source = { scope: "runtimeSnapshot", sourceId: "world-fixture", observedAtUnixMs: 1_788_437_400_000 };
    runtime.bootstrapLauncher.mockResolvedValue(ready);
    runtime.queryLocalGuide.mockResolvedValue({
      entries: [{ id: 17, title: "Épreuve locale", summary: "Description issue du monde de test.", metadata: { level: 5, category: null }, source }],
      provenance: source, uncertainty: "none",
    });
    render(<App />);
    await user.click(await screen.findByRole("button", { name: /réglages/i }));
    await user.click(screen.getByRole("button", { name: /^guide local/i }));
    expect(runtime.queryLocalGuide).not.toHaveBeenCalled();
    expect(screen.getByRole("button", { name: /^rechercher$/i })).toBeDisabled();
    await user.type(screen.getByRole("searchbox", { name: /nom à rechercher/i }), "Épreuve");
    await user.click(screen.getByRole("button", { name: /^rechercher$/i }));
    expect(runtime.queryLocalGuide).toHaveBeenCalledWith("quest", "Épreuve", "frFR");
    expect(await screen.findByRole("heading", { name: "Épreuve locale" })).toBeVisible();
    expect(screen.getByText(/source : références de votre monde/i)).toBeVisible();
    expect(runtime.configureLocalDialogue).not.toHaveBeenCalled();
  });

  it("previews a solo profile before applying it and can restore previous rules", async () => {
    const user = userEvent.setup();
    runtime.bootstrapLauncher.mockResolvedValue(ready);
    render(<App />);
    await user.click(await screen.findByRole("button", { name: /réglages/i }));
    await user.click(screen.getByRole("button", { name: /^profils solo/i }));
    await user.click(await screen.findByRole("radio", { name: /^confort$/i }));
    expect(runtime.configureSoloProfile).not.toHaveBeenCalled();
    expect(screen.getByText("11")).toBeVisible();
    expect(screen.getByText(/pas l’expérience, l’argent ou les métiers déjà acquis/i)).toBeVisible();
    await user.click(screen.getByRole("button", { name: /appliquer ce profil/i }));
    expect(runtime.configureSoloProfile).toHaveBeenCalledWith("comfortable");
    expect(await screen.findByText(/profil enregistré pour la prochaine partie/i)).toBeVisible();
    await user.click(screen.getByRole("button", { name: /revenir aux règles précédentes/i }));
    expect(runtime.rollbackSoloProfile).toHaveBeenCalledTimes(1);
    expect(await screen.findByText(/règles précédentes restaurées/i)).toBeVisible();
  });

  it("blocks solo changes during gameplay and offers an explicit stop", async () => {
    const user = userEvent.setup();
    runtime.bootstrapLauncher.mockResolvedValue(running);
    render(<App />);
    await user.click(await screen.findByRole("button", { name: /réglages/i }));
    await user.click(screen.getByRole("button", { name: /^profils solo/i }));
    expect(await screen.findByRole("radio", { name: /^confort$/i })).toBeDisabled();
    expect(screen.queryByRole("button", { name: /appliquer ce profil/i })).not.toBeInTheDocument();
    await user.click(screen.getByRole("button", { name: /arrêter le monde pour changer les règles/i }));
    expect(runtime.stopRealm).toHaveBeenCalledTimes(1);
    expect(await screen.findByRole("button", { name: /appliquer ce profil/i })).toBeEnabled();
    expect(runtime.configureSoloProfile).not.toHaveBeenCalled();
  });

  it("keeps unknown solo state unavailable instead of presenting a fresh default", async () => {
    const user = userEvent.setup();
    runtime.bootstrapLauncher.mockResolvedValue(ready);
    runtime.inspectSoloProfiles.mockRejectedValue("unknown schema");
    render(<App />);
    await user.click(await screen.findByRole("button", { name: /réglages/i }));
    await user.click(screen.getByRole("button", { name: /^profils solo/i }));
    expect(await screen.findByRole("alert")).toHaveTextContent(/impossible de confirmer/i);
    expect(screen.queryByRole("button", { name: /appliquer ce profil/i })).not.toBeInTheDocument();
    expect(runtime.configureSoloProfile).not.toHaveBeenCalled();
  });

  it("reads back a partially applied solo change without promising rollback", async () => {
    const user = userEvent.setup();
    runtime.bootstrapLauncher.mockResolvedValue(ready);
    runtime.configureSoloProfile.mockRejectedValue("pointer write interrupted");
    render(<App />);
    await user.click(await screen.findByRole("button", { name: /réglages/i }));
    await user.click(screen.getByRole("button", { name: /^profils solo/i }));
    await user.click(await screen.findByRole("radio", { name: /^confort$/i }));
    runtime.inspectSoloProfiles.mockResolvedValue({ ...soloView, activeProfile: "comfortable", pendingChange: true });
    await user.click(screen.getByRole("button", { name: /appliquer ce profil/i }));
    expect(await screen.findByRole("alert")).toHaveTextContent(/un retour arrière n’est pas garanti/i);
    expect(await screen.findByText(/une modification interrompue sera reprise/i)).toBeVisible();
    expect(runtime.inspectSoloProfiles).toHaveBeenCalledTimes(2);
    expect(screen.getByRole("radio", { name: /^confort$/i })).toBeChecked();
    expect(screen.queryByText(/profil enregistré pour la prochaine partie/i)).not.toBeInTheDocument();
  });

  it("distinguishes no guide results from unavailable local references", async () => {
    const user = userEvent.setup();
    runtime.bootstrapLauncher.mockResolvedValue(ready);
    render(<App />);
    await user.click(await screen.findByRole("button", { name: /réglages/i }));
    await user.click(screen.getByRole("button", { name: /^guide local/i }));
    await user.type(screen.getByRole("searchbox", { name: /nom à rechercher/i }), "Absent");
    await user.click(screen.getByRole("button", { name: /^rechercher$/i }));
    expect(await screen.findByText(/aucun résultat pour ce nom/i)).toBeVisible();
    runtime.queryLocalGuide.mockResolvedValue({ entries: [], provenance: null, uncertainty: "unavailable" });
    await user.click(screen.getByRole("button", { name: /^rechercher$/i }));
    expect(await screen.findByRole("alert")).toHaveTextContent(/références locales sont indisponibles/i);
    expect(screen.queryByText(/aucun résultat pour ce nom/i)).not.toBeInTheDocument();
  });

  it("uses English references for the English guide and keeps partial evidence explicit", async () => {
    const user = userEvent.setup();
    localStorage.setItem("realmbox-language", "en");
    runtime.bootstrapLauncher.mockResolvedValue(ready);
    runtime.queryLocalGuide.mockResolvedValue({ entries: [], provenance: null, uncertainty: "partial" });
    render(<App />);
    await user.click(await screen.findByRole("button", { name: /^settings$/i }));
    await user.click(screen.getByRole("button", { name: /^local guide/i }));
    await user.selectOptions(screen.getByRole("combobox", { name: /search for/i }), "item");
    await user.type(screen.getByRole("searchbox", { name: /name to find/i }), "Sword");
    await user.click(screen.getByRole("button", { name: /^search$/i }));
    expect(runtime.queryLocalGuide).toHaveBeenCalledWith("item", "Sword", "enUS");
    expect(await screen.findByText(/partial excerpt/i)).toBeVisible();
  });

  it("lets RealmBox present and activate its CanIRun decision after installation", async () => {
    const user = userEvent.setup();
    runtime.bootstrapLauncher.mockResolvedValue(ready);
    render(<App />);
    await screen.findByRole("button", { name: /jouer/i });

    await user.click(screen.getByRole("button", { name: /réglages/i }));
    await user.click(screen.getByRole("button", { name: /^dialogues/i }));

    expect(screen.getByText("Llama 3.2 3B")).toBeVisible();
    expect(screen.getByText(/2(?:[,.]0)?\s*GB/i)).toBeVisible();
    expect(screen.getByText(/177 tok\/s/i)).toBeVisible();
    expect(screen.getByText(/Llama 3\.2 Community License/i)).toBeVisible();
    await user.click(screen.getByRole("button", { name: /télécharger et activer/i }));
    expect(runtime.configureLocalDialogue).toHaveBeenCalledWith(true, "llama3.2:3b");
    expect(await screen.findByText(/prochain lancement du monde/i)).toBeVisible();
  });

  it("saves a bounded dialogue chattiness while the world is stopped", async () => {
    const user = userEvent.setup();
    runtime.bootstrapLauncher.mockResolvedValue({
      ...ready,
      aiEnabled: true,
      aiModel: "llama3.2:3b",
      dialogueChattiness: "balanced",
    });
    render(<App />);
    await screen.findByRole("button", { name: /^jouer$/i });

    await user.click(screen.getByRole("button", { name: /réglages/i }));
    await user.click(screen.getByRole("button", { name: /dialogues/i }));
    await user.click(screen.getByRole("radio", { name: /vivant · conversations/i }));

    expect(runtime.configureDialogueChattiness).toHaveBeenCalledWith("lively");
    expect(await screen.findByText(/mode de discussion appliqué/i)).toBeVisible();
  });

  it("chooses the next conversation mode while the installed model is disabled", async () => {
    const user = userEvent.setup();
    runtime.bootstrapLauncher.mockResolvedValue({
      ...ready,
      aiEnabled: false,
      aiModel: "llama3.2:3b",
      dialogueChattiness: "balanced",
    });
    runtime.configureDialogueChattiness.mockResolvedValue({
      ...ready,
      aiEnabled: false,
      aiModel: "llama3.2:3b",
      dialogueChattiness: "quiet",
    });
    render(<App />);
    await screen.findByRole("button", { name: /^jouer$/i });

    await user.click(screen.getByRole("button", { name: /réglages/i }));
    await user.click(screen.getByRole("button", { name: /^dialogues/i }));
    await user.click(screen.getByRole("radio", { name: /direct · répond/i }));

    expect(runtime.configureDialogueChattiness).toHaveBeenCalledWith("quiet");
    expect(await screen.findByText(/mode de discussion appliqué/i)).toBeVisible();
  });

  it("reactivates the retained local model without depending on CanIRun", async () => {
    const user = userEvent.setup();
    runtime.bootstrapLauncher.mockResolvedValue({
      ...ready,
      aiEnabled: false,
      aiModel: "llama3.2:3b",
    });
    runtime.inspectAiCapability.mockResolvedValue({
      state: "unavailable",
      deviceName: null,
      ramGb: null,
      modelId: null,
      modelName: null,
      ollamaModel: null,
      grade: null,
      estimatedTokensPerSecond: null,
      downloadSizeGb: null,
      diskAvailableGb: null,
      diskSpaceSufficient: null,
      modelLicense: null,
      detail: "CanIRun indisponible hors ligne.",
      sourceUrl: "https://www.canirun.ai/",
    });
    runtime.configureLocalDialogue.mockResolvedValue({
      ...ready,
      aiEnabled: true,
      aiModel: "llama3.2:3b",
    });
    render(<App />);
    await screen.findByRole("button", { name: /^jouer$/i });

    await user.click(screen.getByRole("button", { name: /réglages/i }));
    await user.click(screen.getByRole("button", { name: /^dialogues/i }));
    const reactivate = await screen.findByRole("button", { name: /réactiver le modèle installé/i });
    expect(reactivate).toBeEnabled();
    await user.click(reactivate);

    expect(runtime.configureLocalDialogue).toHaveBeenCalledWith(true, "llama3.2:3b");
  });

  it("applies dialogue chattiness live when the local model is running", async () => {
    const user = userEvent.setup();
    runtime.bootstrapLauncher.mockResolvedValue({
      ...running,
      aiEnabled: true,
      aiModel: "llama3.2:3b",
      dialogueChattiness: "balanced",
    });
    runtime.configureDialogueChattiness.mockResolvedValue({
      ...running,
      aiEnabled: true,
      aiModel: "llama3.2:3b",
      dialogueChattiness: "lively",
    });
    render(<App />);
    await screen.findByRole("button", { name: /arrêter le monde/i });

    await user.click(screen.getByRole("button", { name: /réglages/i }));
    await user.click(screen.getByRole("button", { name: /^dialogues/i }));
    await user.click(screen.getByRole("radio", { name: /vivant · conversations/i }));

    expect(runtime.configureDialogueChattiness).toHaveBeenCalledWith("lively");
    expect(await screen.findByText(/mode de discussion appliqué/i)).toBeVisible();
  });

  it("changes the installed game folder without reinstalling the realm", async () => {
    const user = userEvent.setup();
    const movedPath = "/Volumes/Jeux/Wrath";
    runtime.bootstrapLauncher.mockResolvedValue(ready);
    runtime.chooseGameData.mockResolvedValue(movedPath);
    runtime.inspectGameData.mockResolvedValue({
      path: movedPath,
      locale: "frFR",
      detail: "Données WotLK frFR reconnues.",
    });
    runtime.changeGameDataPath.mockResolvedValue({ ...ready, gameDataPath: movedPath });
    render(<App />);
    await screen.findByRole("button", { name: /jouer/i });

    await user.click(screen.getByRole("button", { name: /réglages/i }));
    expect(screen.getByText("/Jeux/Wrath")).toBeVisible();
    await user.click(screen.getByRole("button", { name: /changer le dossier du client/i }));

    expect(runtime.inspectGameData).toHaveBeenCalledWith(movedPath);
    expect(runtime.changeGameDataPath).toHaveBeenCalledWith(movedPath);
    expect(await screen.findByText(movedPath)).toBeVisible();
    expect(screen.getByText(/nouveau dossier vérifié et enregistré/i)).toBeVisible();
    expect(runtime.installRealm).not.toHaveBeenCalled();
  });

  it("keeps the client folder locked while the world is running", async () => {
    const user = userEvent.setup();
    runtime.bootstrapLauncher.mockResolvedValue(running);
    render(<App />);
    await screen.findByRole("button", { name: /arrêter le monde/i });

    await user.click(screen.getByRole("button", { name: /réglages/i }));

    expect(screen.getByRole("button", { name: /changer le dossier du client/i })).toBeDisabled();
    expect(screen.getByText(/arrêtez le monde avant de changer/i)).toBeVisible();
  });

  it("explains the running-world blocker and lets the player stop before activation", async () => {
    const user = userEvent.setup();
    runtime.bootstrapLauncher.mockResolvedValue(running);
    render(<App />);
    await screen.findByRole("button", { name: /arrêter le monde/i });

    await user.click(screen.getByRole("button", { name: /réglages/i }));
    await user.click(screen.getByRole("button", { name: /^dialogues/i }));

    expect(screen.getByText(/arrêtez le monde uniquement pour activer ou désactiver/i)).toBeVisible();
    await user.click(screen.getByRole("button", { name: /arrêter le monde pour modifier/i }));
    expect(runtime.stopRealm).toHaveBeenCalledTimes(1);
    expect(await screen.findByRole("button", { name: /télécharger et activer/i })).toBeEnabled();
  });

  it("keeps technical details in the separate diagnostics view", async () => {
    const user = userEvent.setup();
    runtime.bootstrapLauncher.mockResolvedValue({ ...missing, phase: "error", errorCode: "dockerNotRunning", detail: "Docker Desktop doit être démarré: secret detail" });
    render(<App />);
    expect(await screen.findByText(/Docker Desktop n’est pas prêt/i)).toBeVisible();
    expect(screen.queryByText(/secret detail/i)).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /voir le diagnostic/i }));
    await user.click(await screen.findByText(/^cause$/i));
    expect(screen.getByText(/secret detail/i)).toBeVisible();
  });

  it("offers and runs the verified recovery point without opening diagnostics", async () => {
    const user = userEvent.setup();
    runtime.bootstrapLauncher.mockResolvedValue({
      ...ready,
      phase: "error",
      errorCode: "migrationFailed",
      recoveryAvailable: true,
      detail: "migration interrompue",
    });
    render(<App />);

    const restore = await screen.findByRole("button", { name: /restaurer la dernière version fonctionnelle/i });
    await user.click(restore);

    expect(runtime.restoreLastRecovery).toHaveBeenCalledTimes(1);
    expect(await screen.findByRole("button", { name: /jouer/i })).toBeVisible();
  });

  it("resynchronizes restored dialogue preferences before the next launch", async () => {
    const user = userEvent.setup();
    runtime.bootstrapLauncher.mockResolvedValue({ ...ready, aiEnabled: true, aiModel: "llama3.2:3b" });
    runtime.restoreLastRecovery.mockResolvedValue({ ...ready, aiEnabled: false, aiModel: "llama3.2:3b" });
    runtime.bootstrapLauncher
      .mockResolvedValueOnce({ ...ready, phase: "error", errorCode: "migrationFailed", recoveryAvailable: true })
      .mockResolvedValue({ ...ready, aiEnabled: true, aiModel: "llama3.2:3b" });
    render(<App />);

    await user.click(await screen.findByRole("button", { name: /restaurer la dernière version fonctionnelle/i }));
    await user.click(await screen.findByRole("button", { name: /^jouer$/i }));

    expect(runtime.startRealm).toHaveBeenCalledWith(true, 50, "natural", false);
  });

  it("shows immediate feedback while retrying and handles another failure", async () => {
    const user = userEvent.setup();
    let rejectRetry: (reason: unknown) => void = () => undefined;
    runtime.bootstrapLauncher
      .mockResolvedValueOnce({ ...missing, phase: "error", errorCode: "dockerNotRunning", detail: "Docker Desktop doit être démarré" })
      .mockImplementationOnce(() => new Promise((_, reject) => { rejectRetry = reject; }));
    render(<App />);

    const retry = await screen.findByRole("button", { name: /réessayer/i });
    await user.click(retry);

    expect(screen.getByRole("heading", { name: /vérification en cours/i })).toBeVisible();
    expect(screen.getByRole("button", { name: /patientez/i })).toBeDisabled();

    rejectRetry({ code: "dockerNotRunning", component: "launcher", technicalDetail: "moteur indisponible", recoveryActions: ["startDocker", "retry"] });
    expect(await screen.findByRole("button", { name: /réessayer/i })).toBeEnabled();
    expect(screen.getByText(/Docker Desktop n’est pas prêt/i)).toBeVisible();
  });

  it("does not blame Docker Desktop for a later Docker Compose failure", async () => {
    runtime.bootstrapLauncher.mockResolvedValue({
      ...ready,
      phase: "error",
      errorCode: "worldServerTimeout",
      detail: "docker a échoué; voir /runtime/server/logs/start-database.log",
    });
    render(<App />);

    expect(await screen.findByText(/Le serveur local n’est pas prêt/i)).toBeVisible();
    expect(screen.queryByText(/Docker Desktop n’est pas prêt/i)).not.toBeInTheDocument();
  });

  it("keeps Play primary with player-facing realm shortcuts and no diagnostic clutter", async () => {
    runtime.bootstrapLauncher.mockResolvedValue(ready);
    render(<App />);

    expect(await screen.findByRole("button", { name: /jouer/i })).toBeVisible();
    expect(screen.getAllByText("RealmBox")).toHaveLength(1);
    expect(screen.queryByRole("button", { name: /compagnons/i })).not.toBeInTheDocument();
    expect(screen.getByRole("button", { name: /^dialogues$/i })).toBeVisible();
    expect(screen.getByRole("button", { name: /population & présence/i })).toBeVisible();
    expect(screen.getByRole("button", { name: /protéger ma progression/i })).toBeVisible();
    expect(screen.queryByRole("button", { name: /^diagnostic$/i })).not.toBeInTheDocument();
    expect(screen.queryByText(/local uniquement/i)).not.toBeInTheDocument();
    expect(screen.queryByText(/3\.3\.5a/i)).not.toBeInTheDocument();
    expect(screen.queryByRole("progressbar")).not.toBeInTheDocument();
  });

  it("shows progress only while an operation is actually advancing", async () => {
    runtime.bootstrapLauncher.mockResolvedValue({ ...ready, phase: "installing", progress: 42, message: "Préparation de la base locale" });
    render(<App />);

    const progress = await screen.findByRole("progressbar", { name: /progression/i });
    expect(progress).toHaveAttribute("aria-valuenow", "42");
  });

  it("traps focus in the settings dialog and restores the trigger", async () => {
    const user = userEvent.setup();
    runtime.bootstrapLauncher.mockResolvedValue(ready);
    render(<App />);

    const trigger = await screen.findByRole("button", { name: /réglages/i });
    await user.click(trigger);
    const dialog = screen.getByRole("dialog");
    expect(dialog).toContainElement(document.activeElement as HTMLElement);

    await user.keyboard("{Shift>}{Tab}{/Shift}");
    expect(dialog).toContainElement(document.activeElement as HTMLElement);

    await user.keyboard("{Escape}");
    expect(screen.queryByRole("dialog")).not.toBeInTheDocument();
    expect(trigger).toHaveFocus();
  });

  it("omits the local logs path from copied diagnostics", async () => {
    const user = userEvent.setup();
    const writeText = vi.spyOn(navigator.clipboard, "writeText");
    runtime.bootstrapLauncher.mockResolvedValue(ready);
    render(<App />);

    await user.click(await screen.findByRole("button", { name: /réglages/i }));
    await user.click(screen.getByRole("button", { name: /^diagnostic/i }));
    await user.click(await screen.findByRole("button", { name: /copier le diagnostic/i }));

    expect(writeText).toHaveBeenCalledWith(expect.stringContaining("logs=[local path omitted]"));
    expect(writeText).not.toHaveBeenCalledWith(expect.stringContaining("/RealmBox/logs"));
  });

  it("has no automatically detectable accessibility violation in the main flow and settings panels", async () => {
    const user = userEvent.setup();
    runtime.bootstrapLauncher.mockResolvedValue(ready);
    const { container } = render(<App />);
    await screen.findByRole("button", { name: /^jouer$/i });
    const options = { rules: { "color-contrast": { enabled: false } } };
    expect((await axe.run(container, options)).violations).toEqual([]);

    await user.click(screen.getByRole("button", { name: /réglages/i }));
    expect((await axe.run(container, options)).violations).toEqual([]);

    await user.click(screen.getByRole("button", { name: /^compagnons\b/i }));
    expect(await screen.findByRole("heading", { name: /^compagnons\b/i })).toBeVisible();
    expect((await axe.run(container, options)).violations).toEqual([]);

    await user.click(screen.getByRole("button", { name: /retour/i }));
    await user.click(screen.getByRole("button", { name: /^dialogues\b/i }));
    expect(await screen.findByRole("heading", { name: /^dialogues\b/i })).toBeVisible();
    expect((await axe.run(container, options)).violations).toEqual([]);

    await user.click(screen.getByRole("button", { name: /retour/i }));
    await user.click(screen.getByRole("button", { name: /^protection/i }));
    expect(await screen.findByRole("heading", { name: /protection du monde/i })).toBeVisible();
    expect((await axe.run(container, options)).violations).toEqual([]);
    await user.click(screen.getByRole("button", { name: /retour/i }));
    await user.click(screen.getByRole("button", { name: /^guide local/i }));
    expect(await screen.findByRole("heading", { name: /guide de votre monde/i })).toBeVisible();
    expect((await axe.run(container, options)).violations).toEqual([]);
    await user.click(screen.getByRole("button", { name: /retour/i }));
    await user.click(screen.getByRole("button", { name: /^profils solo/i }));
    expect(await screen.findByRole("radio", { name: /^confort$/i })).toBeVisible();
    expect((await axe.run(container, options)).violations).toEqual([]);
  });
});
