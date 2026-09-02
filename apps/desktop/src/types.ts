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

export interface LauncherComponent {
  id: "client" | "database" | "server" | "bots";
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
  gameDataPath: string | null;
  accountName: string | null;
  accountPassword: string | null;
  components: LauncherComponent[];
}

export interface LauncherProgress {
  phase: LauncherPhase;
  message: string;
  detail: string | null;
  progress: number;
}
