import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "../src/App";
import type { LauncherStatus } from "../src/types";

const missing: LauncherStatus = {
  phase: "needsGameData",
  message: "Données de jeu requises",
  detail: null,
  progress: 0,
  installed: false,
  botsEnabled: true,
  botCount: 50,
  aiEnabled: false,
  aiModel: null,
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

const runtime = vi.hoisted(() => ({
  bootstrapLauncher: vi.fn(),
  changeGameDataPath: vi.fn(),
  chooseGameData: vi.fn(),
  configureLocalDialogue: vi.fn(),
  installRealm: vi.fn(),
  startRealm: vi.fn(),
  stopRealm: vi.fn(),
  subscribeLauncherProgress: vi.fn(),
  subscribeLauncherStatus: vi.fn(),
  inspectAiCapability: vi.fn(),
  inspectGameData: vi.fn(),
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
    runtime.startRealm.mockResolvedValue(running);
    runtime.stopRealm.mockResolvedValue(ready);
    runtime.configureLocalDialogue.mockResolvedValue({ ...ready, aiEnabled: true, aiModel: "llama3.2:3b" });
    runtime.updatePlayerbotPopulation.mockResolvedValue({ ...running, botCount: 100 });
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
    expect(screen.getByText(/Data frFR · build 12340/i)).toBeVisible();
    expect(screen.getByRole("button", { name: /^installer$/i })).toBeEnabled();

    await user.click(screen.getByRole("button", { name: /réglages/i }));
    await screen.findByText(/Llama 3\.2 3B/i);
    await user.click(screen.getByRole("checkbox", { name: /dialogues locaux/i }));
    await user.click(screen.getByRole("button", { name: /fermer/i }));
    await user.click(screen.getByRole("button", { name: /^installer$/i }));
    expect(runtime.installRealm).toHaveBeenCalledWith("/Jeux/Wrath", "managedOpenWow", true, 50, true, "llama3.2:3b");
    expect(await screen.findByRole("button", { name: /jouer/i })).toBeVisible();
  });

  it("rejects incomplete game data before installation starts", async () => {
    const user = userEvent.setup();
    runtime.inspectGameData.mockRejectedValue("archive WotLK requise absente : Data/lichking.MPQ");
    render(<App />);

    await screen.findByRole("heading", { name: /préparer mon monde/i });
    await user.click(screen.getByRole("button", { name: /choisir le dossier/i }));

    expect(await screen.findByText(/lichking\.MPQ/i)).toBeVisible();
    expect(screen.queryByRole("button", { name: /^installer$/i })).not.toBeInTheDocument();
    expect(runtime.installRealm).not.toHaveBeenCalled();
  });

  it("surfaces a native folder-picker failure instead of leaving the button inert", async () => {
    const user = userEvent.setup();
    runtime.chooseGameData.mockRejectedValue("dialog.open not allowed");
    render(<App />);

    await screen.findByRole("heading", { name: /préparer mon monde/i });
    await user.click(screen.getByRole("button", { name: /choisir le dossier/i }));

    expect(await screen.findByText(/dialog\.open not allowed/i)).toBeVisible();
    expect(screen.queryByRole("button", { name: /^installer$/i })).not.toBeInTheDocument();
    expect(runtime.inspectGameData).not.toHaveBeenCalled();
  });

  it("persists the player-provided Windows client choice", async () => {
    const user = userEvent.setup();
    render(<App />);

    await screen.findByRole("heading", { name: /préparer mon monde/i });
    await user.click(screen.getByRole("button", { name: /réglages/i }));
    await user.click(screen.getByRole("radio", { name: /mon client original/i }));
    await user.click(screen.getByRole("button", { name: /fermer/i }));
    await user.click(screen.getByRole("button", { name: /choisir le dossier/i }));
    await user.click(screen.getByRole("button", { name: /^installer$/i }));

    expect(runtime.installRealm).toHaveBeenCalledWith(
      "/Jeux/Wrath",
      "originalWindows",
      true,
      50,
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

    await userEvent.setup().click(await screen.findByRole("button", { name: /réglages/i }));
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

    expect(screen.getByRole("heading", { name: /set up my world/i })).toBeVisible();
    expect(screen.getByText(/does not download proprietary game data/i)).toBeVisible();
    await user.click(screen.getByRole("button", { name: /close/i }));
    expect(screen.getByRole("button", { name: /choose the folder/i })).toBeVisible();
  });

  it("applies a running bot population without stopping the client", async () => {
    const user = userEvent.setup();
    runtime.bootstrapLauncher.mockResolvedValue(running);
    render(<App />);
    await screen.findByRole("button", { name: /arrêter le monde/i });

    await user.click(screen.getByRole("button", { name: /réglages/i }));
    await user.click(screen.getByRole("button", { name: /compagnons/i }));
    expect(screen.getByRole("heading", { name: /population du monde/i })).toBeVisible();
    await user.selectOptions(screen.getByRole("combobox"), "100");
    await user.click(screen.getByRole("button", { name: /appliquer maintenant/i }));

    expect(runtime.updatePlayerbotPopulation).toHaveBeenCalledWith(true, 100);
    expect(runtime.stopRealm).not.toHaveBeenCalled();
    expect(await screen.findByText(/sans redémarrer le client/i)).toBeVisible();
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

    expect(screen.getByText(/monde doit être arrêté/i)).toBeVisible();
    await user.click(screen.getByRole("button", { name: /arrêter le monde pour continuer/i }));
    expect(runtime.stopRealm).toHaveBeenCalledTimes(1);
    expect(await screen.findByRole("button", { name: /télécharger et activer/i })).toBeEnabled();
  });

  it("keeps technical details in the separate diagnostics view", async () => {
    const user = userEvent.setup();
    runtime.bootstrapLauncher.mockResolvedValue({ ...missing, phase: "error", detail: "Docker Desktop doit être démarré: secret detail" });
    render(<App />);
    expect(await screen.findByText(/Docker Desktop n’est pas prêt/i)).toBeVisible();
    expect(screen.queryByText(/secret detail/i)).not.toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: /voir le diagnostic/i }));
    await user.click(await screen.findByText(/^cause$/i));
    expect(screen.getByText(/secret detail/i)).toBeVisible();
  });

  it("shows immediate feedback while retrying and handles another failure", async () => {
    const user = userEvent.setup();
    let rejectRetry: (reason: string) => void = () => undefined;
    runtime.bootstrapLauncher
      .mockResolvedValueOnce({ ...missing, phase: "error", detail: "Docker Desktop doit être démarré" })
      .mockImplementationOnce(() => new Promise((_, reject) => { rejectRetry = reject; }));
    render(<App />);

    const retry = await screen.findByRole("button", { name: /réessayer/i });
    await user.click(retry);

    expect(screen.getByRole("heading", { name: /vérification en cours/i })).toBeVisible();
    expect(screen.getByRole("button", { name: /patientez/i })).toBeDisabled();

    rejectRetry("Docker Desktop doit être démarré: moteur indisponible");
    expect(await screen.findByRole("button", { name: /réessayer/i })).toBeEnabled();
    expect(screen.getByText(/Docker Desktop n’est pas prêt/i)).toBeVisible();
  });

  it("does not blame Docker Desktop for a later Docker Compose failure", async () => {
    runtime.bootstrapLauncher.mockResolvedValue({
      ...ready,
      phase: "error",
      detail: "docker a échoué; voir /runtime/server/logs/start-database.log",
    });
    render(<App />);

    expect(await screen.findByText(/Le serveur local n’est pas prêt/i)).toBeVisible();
    expect(screen.queryByText(/Docker Desktop n’est pas prêt/i)).not.toBeInTheDocument();
  });

  it("keeps the home screen launcher-like and hides secondary features", async () => {
    runtime.bootstrapLauncher.mockResolvedValue(ready);
    render(<App />);

    expect(await screen.findByRole("button", { name: /jouer/i })).toBeVisible();
    expect(screen.getAllByText("RealmBox")).toHaveLength(1);
    expect(screen.queryByRole("button", { name: /compagnons/i })).not.toBeInTheDocument();
    expect(screen.queryByRole("button", { name: /^dialogues$/i })).not.toBeInTheDocument();
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
});
