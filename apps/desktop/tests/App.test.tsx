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
  aiEnabled: false,
  aiModel: null,
  gameDataPath: null,
  accountName: null,
  accountPassword: null,
  components: [
    { id: "client", label: "Client de jeu", state: "missing", detail: "À préparer" },
    { id: "database", label: "Sauvegarde du royaume", state: "missing", detail: "À préparer" },
    { id: "server", label: "Monde privé", state: "missing", detail: "À préparer" },
    { id: "bots", label: "Compagnons", state: "missing", detail: "Optionnels" },
    { id: "ai", label: "Dialogues vivants", state: "stopped", detail: "Selon ce Mac" },
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
  chooseGameData: vi.fn(),
  installRealm: vi.fn(),
  startRealm: vi.fn(),
  stopRealm: vi.fn(),
  subscribeLauncherProgress: vi.fn(),
  subscribeLauncherStatus: vi.fn(),
  inspectAiCapability: vi.fn(),
}));

vi.mock("../src/runtime", () => runtime);

describe("RealmBox launcher", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    runtime.bootstrapLauncher.mockResolvedValue(missing);
    runtime.chooseGameData.mockResolvedValue("/Jeux/Wrath");
    runtime.installRealm.mockResolvedValue(ready);
    runtime.startRealm.mockResolvedValue(running);
    runtime.stopRealm.mockResolvedValue(ready);
    runtime.subscribeLauncherProgress.mockResolvedValue(() => undefined);
    runtime.subscribeLauncherStatus.mockResolvedValue(() => undefined);
    runtime.inspectAiCapability.mockResolvedValue({
      state: "recommended",
      deviceName: "Apple M4 Max",
      ramGb: 36,
      modelId: "qwen3-8b",
      modelName: "Qwen 3 8B",
      ollamaModel: "qwen3:8b",
      grade: "S",
      estimatedTokensPerSecond: 77,
      detail: "CanIRun le classe confortable.",
      sourceUrl: "https://www.canirun.ai/",
    });
  });

  it("requires owned game data before the real installation", async () => {
    const user = userEvent.setup();
    render(<App />);

    expect(await screen.findByRole("heading", { name: /données de jeu requises/i })).toBeVisible();
    const install = screen.getByRole("button", { name: /installer/i });
    expect(install).toBeDisabled();

    await user.click(screen.getByRole("button", { name: /parcourir/i }));
    expect(screen.getByText("/Jeux/Wrath")).toBeVisible();
    expect(install).toBeEnabled();

    await user.click(install);
    expect(runtime.installRealm).toHaveBeenCalledWith("/Jeux/Wrath", true, true, "qwen3:8b");
    expect(await screen.findByRole("button", { name: /jouer/i })).toBeVisible();
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
      detail: "Mémoire insuffisante.",
      sourceUrl: "https://www.canirun.ai/",
    });
    render(<App />);

    expect(await screen.findByText(/aucun modèle confortable/i)).toBeVisible();
    expect(screen.getByRole("checkbox", { name: /dialogues IA/i })).toBeDisabled();
  });

  it("renders the already-started result returned on a later launch", async () => {
    runtime.bootstrapLauncher.mockResolvedValue(running);
    render(<App />);

    expect(await screen.findByRole("heading", { name: /le monde est lancé/i })).toBeVisible();
    expect(screen.getByRole("button", { name: /arrêter/i })).toBeVisible();
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
    expect(await screen.findByRole("button", { name: /arrêter/i })).toBeVisible();

    act(() => publishStatus?.(ready));
    expect(await screen.findByRole("button", { name: /jouer/i })).toBeVisible();
  });
});
