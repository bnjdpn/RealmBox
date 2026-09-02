export type LauncherPhase =
  | "checking"
  | "needsGameData"
  | "installing"
  | "ready"
  | "starting"
  | "running"
  | "stopping"
  | "error";

export type ComponentState = "missing" | "installing" | "ready" | "running" | "stopped" | "error";
export type ClientChoice = "managedOpenWow" | "originalWindows";

export interface LauncherComponent {
  id: "client" | "database" | "server" | "bots" | "ai";
  label: string;
  state: ComponentState;
  detail: string;
}

export interface LauncherStatus {
  phase: LauncherPhase;
  message: string;
  detail: string | null;
  progress: number;
  installed: boolean;
  botsEnabled: boolean;
  aiEnabled: boolean;
  aiModel: string | null;
  gameDataPath: string | null;
  accountName: string | null;
  accountPassword: string | null;
  clientChoice: ClientChoice;
  originalClientSupported: boolean;
  platformLabel: string;
  components: LauncherComponent[];
}

export interface AiCapability {
  state: "checking" | "recommended" | "unavailable";
  deviceName: string | null;
  ramGb: number | null;
  modelId: string | null;
  modelName: string | null;
  ollamaModel: string | null;
  grade: string | null;
  estimatedTokensPerSecond: number | null;
  detail: string;
  sourceUrl: string;
}

export interface LauncherProgress {
  phase: LauncherPhase;
  message: string;
  detail: string | null;
  progress: number;
}
