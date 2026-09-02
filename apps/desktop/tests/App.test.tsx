import { render, screen } from "@testing-library/react";
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
  gameDataPath: null,
  accountName: null,
  accountPassword: null,
  components: [
    { id: "client", label: "Client de jeu", state: "missing", detail: "À préparer" },
    { id: "database", label: "Sauvegarde du royaume", state: "missing", detail: "À préparer" },
    { id: "server", label: "Monde privé", state: "missing", detail: "À préparer" },
    { id: "bots", label: "Compagnons", state: "missing", detail: "Optionnels" },
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
    expect(runtime.installRealm).toHaveBeenCalledWith("/Jeux/Wrath", true);
    expect(await screen.findByRole("button", { name: /jouer/i })).toBeVisible();
  });

  it("renders the already-started result returned on a later launch", async () => {
    runtime.bootstrapLauncher.mockResolvedValue(running);
    render(<App />);

    expect(await screen.findByRole("heading", { name: /le monde est lancé/i })).toBeVisible();
    expect(screen.getByRole("button", { name: /arrêter/i })).toBeVisible();
    expect(screen.queryByRole("button", { name: /installer/i })).not.toBeInTheDocument();
  });
});
