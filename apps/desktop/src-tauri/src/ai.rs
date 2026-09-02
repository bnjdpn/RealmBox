use std::ffi::OsString;

use serde::{Deserialize, Serialize};

use crate::launcher::CommandRunner;

const CANIRUN_COMPATIBILITY_URL: &str = "https://www.canirun.ai/api/compatibility";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AiCapabilityState {
    Recommended,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiCapability {
    pub state: AiCapabilityState,
    pub device_name: Option<String>,
    pub ram_gb: Option<u32>,
    pub model_id: Option<String>,
    pub model_name: Option<String>,
    pub ollama_model: Option<String>,
    pub grade: Option<String>,
    pub estimated_tokens_per_second: Option<u32>,
    pub detail: String,
    pub source_url: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct AiCandidate {
    pub model_id: &'static str,
    pub model_name: &'static str,
    pub ollama_model: &'static str,
}

pub const AI_CANDIDATES: [AiCandidate; 4] = [
    AiCandidate {
        model_id: "qwen3-8b",
        model_name: "Qwen 3 8B",
        ollama_model: "qwen3:8b",
    },
    AiCandidate {
        model_id: "gemma3-4b",
        model_name: "Gemma 3 4B",
        ollama_model: "gemma3:4b",
    },
    AiCandidate {
        model_id: "llama3.2-3b",
        model_name: "Llama 3.2 3B",
        ollama_model: "llama3.2:3b",
    },
    AiCandidate {
        model_id: "llama3.2-1b",
        model_name: "Llama 3.2 1B",
        ollama_model: "llama3.2:1b",
    },
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CompatibilityRequest<'a> {
    hardware: HardwareRequest<'a>,
    model_id: &'a str,
    quantization: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct HardwareRequest<'a> {
    cpu: CpuRequest<'a>,
    ram_gb: u32,
    gpu: GpuRequest<'a>,
}

#[derive(Debug, Serialize)]
struct CpuRequest<'a> {
    name: &'a str,
    cores: u32,
    threads: u32,
}

#[derive(Debug, Serialize)]
struct GpuRequest<'a> {
    name: &'a str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LocalHardware {
    device_name: String,
    logical_cores: u32,
    memory_bytes: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompatibilityResponse {
    compatible: bool,
    status: String,
    grade: String,
    estimated: CompatibilityEstimate,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CompatibilityEstimate {
    tokens_per_second: f64,
    ram_required_gb: f64,
}

pub fn inspect_ai_capability<R: CommandRunner>(runner: &R) -> AiCapability {
    inspect_ai_capability_result(runner).unwrap_or_else(|error| AiCapability {
        state: AiCapabilityState::Unavailable,
        device_name: None,
        ram_gb: None,
        model_id: None,
        model_name: None,
        ollama_model: None,
        grade: None,
        estimated_tokens_per_second: None,
        detail: format!("Conseil CanIRun indisponible : {error}"),
        source_url: "https://www.canirun.ai/",
    })
}

fn inspect_ai_capability_result<R: CommandRunner>(runner: &R) -> Result<AiCapability, String> {
    let hardware = inspect_local_hardware(runner)?;
    let device_name = hardware.device_name;
    let cores = hardware.logical_cores;
    let memory_bytes = hardware.memory_bytes;
    let ram_gb = (memory_bytes / 1024 / 1024 / 1024) as u32;
    if ram_gb < 16 {
        return Ok(AiCapability {
            state: AiCapabilityState::Unavailable,
            device_name: Some(device_name),
            ram_gb: Some(ram_gb),
            model_id: None,
            model_name: None,
            ollama_model: None,
            grade: None,
            estimated_tokens_per_second: None,
            detail: "L’IA de dialogue reste désactivée : RealmBox réserve cette option aux machines disposant d’au moins 16 Go de mémoire.".into(),
            source_url: "https://www.canirun.ai/",
        });
    }

    let ai_budget_gb = f64::from(ram_gb).mul_add(0.25, 0.0).min(8.0);
    let mut compatible = Vec::new();
    let mut reports_received = 0_u8;
    for candidate in AI_CANDIDATES {
        let request = CompatibilityRequest {
            hardware: HardwareRequest {
                cpu: CpuRequest {
                    name: &device_name,
                    cores,
                    threads: cores,
                },
                ram_gb,
                gpu: GpuRequest { name: &device_name },
            },
            model_id: candidate.model_id,
            quantization: "Q4_K_M",
        };
        let body = serde_json::to_string(&request).map_err(|error| error.to_string())?;
        let Ok(response) = runner.run(
            "curl",
            &[
                "--silent".into(),
                "--show-error".into(),
                "--fail".into(),
                "--max-time".into(),
                "8".into(),
                "--request".into(),
                "POST".into(),
                CANIRUN_COMPATIBILITY_URL.into(),
                "--header".into(),
                "content-type: application/json".into(),
                "--data".into(),
                OsString::from(body),
            ],
            None,
        ) else {
            continue;
        };
        let Ok(report) = serde_json::from_str::<CompatibilityResponse>(&response) else {
            continue;
        };
        reports_received += 1;
        if report.compatible
            && report.status == "comfortable"
            && report.estimated.ram_required_gb <= ai_budget_gb
        {
            compatible.push((candidate, report));
        }
    }

    if reports_received == 0 {
        return Err("aucune réponse CanIRun exploitable".into());
    }

    // Candidates are deliberately ordered from the richest model to the lightest.
    // CanIRun decides whether each one fits; RealmBox then keeps the first that also
    // respects the memory reserved for the game and its server.
    let Some((candidate, report)) = compatible.into_iter().next() else {
        return Ok(AiCapability {
            state: AiCapabilityState::Unavailable,
            device_name: Some(device_name),
            ram_gb: Some(ram_gb),
            model_id: None,
            model_name: None,
            ollama_model: None,
            grade: None,
            estimated_tokens_per_second: None,
            detail: "CanIRun ne classe aucun modèle RealmBox comme confortable dans le budget réservé au jeu.".into(),
            source_url: "https://www.canirun.ai/",
        });
    };

    Ok(AiCapability {
        state: AiCapabilityState::Recommended,
        device_name: Some(device_name),
        ram_gb: Some(ram_gb),
        model_id: Some(candidate.model_id.into()),
        model_name: Some(candidate.model_name.into()),
        ollama_model: Some(candidate.ollama_model.into()),
        grade: Some(report.grade),
        estimated_tokens_per_second: Some(report.estimated.tokens_per_second.round() as u32),
        detail: format!(
            "CanIRun le classe confortable en Q4 ; RealmBox limite l’IA à {:.0} Go pour laisser de la mémoire au monde.",
            ai_budget_gb
        ),
        source_url: "https://www.canirun.ai/",
    })
}

fn inspect_local_hardware<R: CommandRunner>(runner: &R) -> Result<LocalHardware, String> {
    #[cfg(target_os = "macos")]
    return Ok(LocalHardware {
        device_name: runner.run(
            "sysctl",
            &["-n".into(), "machdep.cpu.brand_string".into()],
            None,
        )?,
        logical_cores: parse_u32(
            &runner.run("sysctl", &["-n".into(), "hw.ncpu".into()], None)?,
            "cœurs",
        )?,
        memory_bytes: runner
            .run("sysctl", &["-n".into(), "hw.memsize".into()], None)?
            .parse::<u64>()
            .map_err(|_| "mémoire système illisible".to_string())?,
    });

    #[cfg(windows)]
    return Ok(LocalHardware {
        device_name: runner.run(
            "powershell.exe",
            &[
                "-NoProfile".into(),
                "-NonInteractive".into(),
                "-Command".into(),
                "(Get-CimInstance Win32_Processor | Select-Object -First 1 -ExpandProperty Name)"
                    .into(),
            ],
            None,
        )?,
        logical_cores: parse_u32(
            &runner.run(
                "powershell.exe",
                &[
                    "-NoProfile".into(),
                    "-NonInteractive".into(),
                    "-Command".into(),
                    "(Get-CimInstance Win32_ComputerSystem).NumberOfLogicalProcessors".into(),
                ],
                None,
            )?,
            "cœurs",
        )?,
        memory_bytes: runner
            .run(
                "powershell.exe",
                &[
                    "-NoProfile".into(),
                    "-NonInteractive".into(),
                    "-Command".into(),
                    "(Get-CimInstance Win32_ComputerSystem).TotalPhysicalMemory".into(),
                ],
                None,
            )?
            .parse::<u64>()
            .map_err(|_| "mémoire système illisible".to_string())?,
    });

    #[allow(unreachable_code)]
    Err("inspection matérielle non prise en charge sur cette plateforme".into())
}

pub fn is_allowed_ollama_model(model: &str) -> bool {
    AI_CANDIDATES
        .iter()
        .any(|candidate| candidate.ollama_model == model)
}

fn parse_u32(value: &str, label: &str) -> Result<u32, String> {
    value
        .trim()
        .parse::<u32>()
        .map_err(|_| format!("{label} illisibles"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{path::Path, sync::Mutex, time::Duration};

    #[derive(Default)]
    struct FakeRunner {
        requests: Mutex<Vec<String>>,
    }

    impl CommandRunner for FakeRunner {
        fn run(
            &self,
            program: &str,
            args: &[OsString],
            _current_dir: Option<&Path>,
        ) -> Result<String, String> {
            if program == "sysctl" {
                return match args.last().and_then(|arg| arg.to_str()) {
                    Some("machdep.cpu.brand_string") => Ok("Apple M4 Max".into()),
                    Some("hw.ncpu") => Ok("14".into()),
                    Some("hw.memsize") => Ok((36_u64 * 1024 * 1024 * 1024).to_string()),
                    _ => Err("sysctl inattendu".into()),
                };
            }
            if program == "curl" {
                let body = args
                    .last()
                    .and_then(|arg| arg.to_str())
                    .ok_or_else(|| "corps absent".to_string())?
                    .to_owned();
                self.requests.lock().expect("requests").push(body.clone());
                let comfortable = body.contains("qwen3-8b");
                return Ok(format!(
                    r#"{{"compatible":{comfortable},"status":"{}","grade":"S","estimated":{{"tokensPerSecond":77.2,"ramRequiredGb":4.6}}}}"#,
                    if comfortable {
                        "comfortable"
                    } else {
                        "not_recommended"
                    }
                ));
            }
            Err(format!("commande inattendue: {program}"))
        }

        fn run_long(
            &self,
            _program: &str,
            _args: &[OsString],
            _current_dir: Option<&Path>,
            _log_path: &Path,
        ) -> Result<(), String> {
            unreachable!()
        }

        fn run_long_with_env(
            &self,
            _program: &Path,
            _args: &[OsString],
            _environment: &[(OsString, OsString)],
            _current_dir: Option<&Path>,
            _log_path: &Path,
        ) -> Result<(), String> {
            unreachable!()
        }

        fn spawn(
            &self,
            _program: &Path,
            _args: &[OsString],
            _environment: &[(OsString, OsString)],
            _current_dir: Option<&Path>,
            _log_path: &Path,
        ) -> Result<u32, String> {
            unreachable!()
        }

        fn terminate(&self, _process_id: u32) -> Result<(), String> {
            unreachable!()
        }

        fn is_process_running(&self, _process_id: u32) -> Result<bool, String> {
            unreachable!()
        }

        fn wait_tcp(&self, _port: u16, _timeout: Duration) -> Result<(), String> {
            unreachable!()
        }
    }

    #[test]
    fn model_allowlist_rejects_arbitrary_ollama_names() {
        assert!(is_allowed_ollama_model("qwen3:8b"));
        assert!(!is_allowed_ollama_model("registry.invalid/model:latest"));
    }

    #[test]
    fn recommendation_uses_only_coarse_hardware_and_keeps_game_memory_free() {
        let runner = FakeRunner::default();
        let capability = inspect_ai_capability(&runner);
        assert_eq!(capability.state, AiCapabilityState::Recommended);
        assert_eq!(capability.ollama_model.as_deref(), Some("qwen3:8b"));
        assert_eq!(capability.estimated_tokens_per_second, Some(77));
        let requests = runner.requests.lock().expect("requests");
        assert_eq!(requests.len(), AI_CANDIDATES.len());
        assert!(requests[0].contains(r#""ramGb":36"#));
        assert!(!requests[0].contains("serial"));
        assert!(!requests[0].contains("username"));
    }
}
