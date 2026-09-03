export type LauncherPhase =
  | "checking"
  | "needsGameData"
  | "installing"
  | "ready"
  | "starting"
  | "running"
  | "stopping"
  | "recovering"
  | "error";

export type ComponentState = "missing" | "installing" | "ready" | "running" | "stopped" | "error";
export type ClientChoice = "managedOpenWow" | "originalWindows";
export type DialogueChattiness = "quiet" | "balanced" | "lively";
export type BotPresence = "dispersed" | "natural" | "close";
export type LauncherErrorCode =
  | "dockerMissing"
  | "dockerNotRunning"
  | "portUnavailable"
  | "gameDataIncomplete"
  | "gameBuildUnsupported"
  | "downloadInterrupted"
  | "checksumMismatch"
  | "backupFailed"
  | "migrationFailed"
  | "recoveryFailed"
  | "clientLaunchFailed"
  | "worldServerTimeout"
  | "installationIncomplete"
  | "installationStateUnreadable"
  | "operationUnavailable"
  | "unknown";

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
  errorCode: LauncherErrorCode | null;
  progress: number;
  installed: boolean;
  recoveryAvailable: boolean;
  botsEnabled: boolean;
  botCount: number;
  requestedBotCount: number;
  appliedBotCount: number;
  botPresence: BotPresence;
  aiEnabled: boolean;
  aiModel: string | null;
  dialogueChattiness: DialogueChattiness;
  gameDataPath: string | null;
  accountName: string | null;
  accountPassword: string | null;
  clientChoice: ClientChoice;
  originalClientSupported: boolean;
  platformLabel: string;
  components: LauncherComponent[];
  operationId?: string;
  component?: LauncherProgress["component"];
  step?: LauncherProgress["step"];
  completedBytes?: number | null;
  totalBytes?: number | null;
  cancellable?: boolean;
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
  downloadSizeGb: number | null;
  diskAvailableGb: number | null;
  diskSpaceSufficient: boolean | null;
  modelLicense: string | null;
  detail: string;
  sourceUrl: string;
}

export interface GameDataInspection {
  path: string;
  locale: string;
  detail: string;
}

export interface LauncherProgress {
  operationId: string;
  component: "launcher" | "gameData" | "client" | "server" | "database" | "bots" | "ai";
  step: "validate" | "download" | "verify" | "extract" | "configure" | "start" | "stop" | "restore" | "complete";
  phase: LauncherPhase;
  message: string;
  detail: string | null;
  errorCode: LauncherErrorCode | null;
  progress: number;
  completedBytes: number | null;
  totalBytes: number | null;
  cancellable: boolean;
}

export interface LauncherCommandError {
  code: LauncherErrorCode;
  component: "client" | "database" | "server" | "bots" | "ai" | "launcher";
  technicalDetail: string | null;
  recoveryActions: Array<"retry" | "chooseGameData" | "startDocker" | "openDiagnostics">;
}

export interface RealmDiagnostics {
  summary: string;
  component: "client" | "database" | "server" | "bots" | "ai" | "launcher";
  logsPath: string;
  recentEntries: string[];
}

export interface RealmBackupSummary {
  createdAtUnixMs: number;
  sizeBytes: number;
}
