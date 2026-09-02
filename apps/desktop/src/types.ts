export type WorldPreset = "calm" | "living" | "crowded";
export type AppView = "welcome" | "data" | "experience" | "preparing" | "dashboard" | "starting" | "running";

export interface Companion {
  id: string;
  name: string;
  role: "tank" | "healer" | "damage";
  level: number;
  ready: boolean;
}

export interface Dashboard {
  playerName: string;
  className: string;
  level: number;
  preset: WorldPreset;
  localAiReady: boolean;
  companions: Companion[];
  sessionRunning: boolean;
  evidence: string;
}

export const fakeDashboard: Dashboard = {
  playerName: "Benjamin",
  className: "Paladin",
  level: 17,
  preset: "living",
  localAiReady: true,
  sessionRunning: false,
  evidence: "Mode démonstration — aucun service réel ni donnée de jeu",
  companions: [
    { id: "thoran", name: "Thoran", role: "tank", level: 17, ready: true },
    { id: "melya", name: "Melya", role: "healer", level: 17, ready: true },
    { id: "kael", name: "Kael", role: "damage", level: 16, ready: true },
    { id: "lyra", name: "Lyra", role: "damage", level: 18, ready: true },
  ],
};

