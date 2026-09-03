use std::time::Duration;

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
    pub download_size_gb: Option<f32>,
    pub disk_available_gb: Option<f32>,
    pub disk_space_sufficient: Option<bool>,
    pub model_license: Option<&'static str>,
    pub detail: String,
    pub source_url: &'static str,
}

#[derive(Debug, Clone, Copy)]
pub struct AiCandidate {
    pub model_id: &'static str,
    pub model_name: &'static str,
    pub ollama_model: &'static str,
    pub download_size_gb: f32,
    pub ollama_digest: &'static str,
    pub license: &'static str,
    pub fallback_only: bool,
}

pub const AI_CANDIDATES: [AiCandidate; 4] = [
    AiCandidate {
        model_id: "qwen3-8b",
        model_name: "Qwen 3 8B",
        ollama_model: "qwen3:8b",
        download_size_gb: 5.2,
        ollama_digest: "500a1f067a9f782620b40bee6f7b0c89e17ae61f686b92c24933e4ca4b2b8b41",
        license: "Apache-2.0",
        fallback_only: false,
    },
    AiCandidate {
        model_id: "gemma3-4b",
        model_name: "Gemma 3 4B",
        ollama_model: "gemma3:4b",
        download_size_gb: 3.3,
        ollama_digest: "a2af6cc3eb7fa8be8504abaf9b04e88f17a119ec3f04a3addf55f92841195f5a",
        license: "Gemma Terms of Use",
        fallback_only: false,
    },
    AiCandidate {
        model_id: "llama3.2-3b",
        model_name: "Llama 3.2 3B",
        ollama_model: "llama3.2:3b",
        download_size_gb: 2.0,
        ollama_digest: "a80c4f17acd55265feec403c7aef86be0c25983ab279d83f3bcd3abbcb5b8b72",
        license: "Llama 3.2 Community License",
        fallback_only: false,
    },
    AiCandidate {
        model_id: "llama3.2-1b",
        model_name: "Llama 3.2 1B",
        ollama_model: "llama3.2:1b",
        download_size_gb: 1.3,
        ollama_digest: "baf6a787fdffd633537aa2eb51cfd54cb93ff08e28040095462bb63daf552878",
        license: "Llama 3.2 Community License",
        fallback_only: true,
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
        download_size_gb: None,
        disk_available_gb: None,
        disk_space_sufficient: None,
        model_license: None,
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
            download_size_gb: None,
            disk_available_gb: None,
            disk_space_sufficient: None,
            model_license: None,
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
        let Ok(response) =
            runner.post_json(CANIRUN_COMPATIBILITY_URL, &body, Duration::from_secs(8))
        else {
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

    // RealmBox makes the decision at runtime from CanIRun's measurements. It prefers
    // dialogue-capable 3B+ candidates and maximizes estimated throughput per official
    // Ollama download GB. The 1B candidate remains an automatic fallback for machines
    // on which CanIRun rejects every larger candidate.
    let recommended = compatible
        .iter()
        .filter(|(candidate, _)| !candidate.fallback_only)
        .max_by(
            |(left_candidate, left_report), (right_candidate, right_report)| {
                let left_score = left_report.estimated.tokens_per_second
                    / f64::from(left_candidate.download_size_gb);
                let right_score = right_report.estimated.tokens_per_second
                    / f64::from(right_candidate.download_size_gb);
                left_score.total_cmp(&right_score)
            },
        )
        .or_else(|| {
            compatible.iter().max_by(
                |(left_candidate, left_report), (right_candidate, right_report)| {
                    let left_score = left_report.estimated.tokens_per_second
                        / f64::from(left_candidate.download_size_gb);
                    let right_score = right_report.estimated.tokens_per_second
                        / f64::from(right_candidate.download_size_gb);
                    left_score.total_cmp(&right_score)
                },
            )
        });
    let Some((candidate, report)) = recommended else {
        return Ok(AiCapability {
            state: AiCapabilityState::Unavailable,
            device_name: Some(device_name),
            ram_gb: Some(ram_gb),
            model_id: None,
            model_name: None,
            ollama_model: None,
            grade: None,
            estimated_tokens_per_second: None,
            download_size_gb: None,
            disk_available_gb: None,
            disk_space_sufficient: None,
            model_license: None,
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
        grade: Some(report.grade.clone()),
        estimated_tokens_per_second: Some(report.estimated.tokens_per_second.round() as u32),
        download_size_gb: Some(candidate.download_size_gb),
        disk_available_gb: None,
        disk_space_sufficient: None,
        model_license: Some(candidate.license),
        detail: format!(
            "CanIRun le classe confortable en Q4 ; RealmBox l’a choisi automatiquement pour son rapport vitesse/taille dans un budget de {:.0} Go.",
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

pub fn model_download_bytes(model: &str) -> Option<u64> {
    AI_CANDIDATES
        .iter()
        .find(|candidate| candidate.ollama_model == model)
        .map(|candidate| (f64::from(candidate.download_size_gb) * 1024_f64.powi(3)) as u64)
}

pub fn expected_ollama_digest(model: &str) -> Option<&'static str> {
    AI_CANDIDATES
        .iter()
        .find(|candidate| candidate.ollama_model == model)
        .map(|candidate| candidate.ollama_digest)
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
    use std::{ffi::OsString, path::Path, sync::Mutex, time::Duration};

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
            if program == "powershell.exe" {
                let command = args
                    .last()
                    .and_then(|arg| arg.to_str())
                    .ok_or_else(|| "commande PowerShell absente".to_string())?;
                if command.contains("Win32_Processor") {
                    return Ok("Apple M4 Max".into());
                }
                if command.contains("NumberOfLogicalProcessors") {
                    return Ok("14".into());
                }
                if command.contains("TotalPhysicalMemory") {
                    return Ok((36_u64 * 1024 * 1024 * 1024).to_string());
                }
                return Err("commande PowerShell inattendue".into());
            }
            if program == "curl" {
                let body = args
                    .last()
                    .and_then(|arg| arg.to_str())
                    .ok_or_else(|| "corps absent".to_string())?
                    .to_owned();
                self.requests.lock().expect("requests").push(body.clone());
                let (comfortable, tokens, ram) = if body.contains("qwen3-8b") {
                    (true, 77.0, 7.5)
                } else if body.contains("gemma3-4b") {
                    (true, 142.0, 3.7)
                } else if body.contains("llama3.2-3b") {
                    (true, 177.0, 2.8)
                } else if body.contains("llama3.2-1b") {
                    (true, 355.0, 2.0)
                } else {
                    (false, 0.0, 0.0)
                };
                return Ok(format!(
                    r#"{{"compatible":{comfortable},"status":"{}","grade":"S","estimated":{{"tokensPerSecond":{tokens},"ramRequiredGb":{ram}}}}}"#,
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

        fn wait_service_tcp(
            &self,
            _compose_file: &Path,
            _service: &str,
            _port: u16,
            _timeout: Duration,
        ) -> Result<(), String> {
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
        assert_eq!(
            expected_ollama_digest("llama3.2:3b"),
            Some("a80c4f17acd55265feec403c7aef86be0c25983ab279d83f3bcd3abbcb5b8b72")
        );
    }

    #[test]
    fn compiled_allowlist_matches_the_reviewed_model_catalog() {
        let catalog = include_str!("../../../../runtime/model-catalog.toml");
        for candidate in AI_CANDIDATES {
            assert!(catalog.contains(candidate.model_id));
            assert!(catalog.contains(candidate.ollama_model));
            assert!(catalog.contains(candidate.ollama_digest));
            assert!(catalog.contains(candidate.license));
        }
    }

    #[test]
    fn recommendation_uses_only_coarse_hardware_and_keeps_game_memory_free() {
        let runner = FakeRunner::default();
        let capability = inspect_ai_capability(&runner);
        assert_eq!(capability.state, AiCapabilityState::Recommended);
        assert_eq!(capability.ollama_model.as_deref(), Some("llama3.2:3b"));
        assert_eq!(capability.download_size_gb, Some(2.0));
        assert_eq!(capability.estimated_tokens_per_second, Some(177));
        let requests = runner.requests.lock().expect("requests");
        assert_eq!(requests.len(), AI_CANDIDATES.len());
        assert!(requests[0].contains(r#""ramGb":36"#));
        assert!(!requests[0].contains("serial"));
        assert!(!requests[0].contains("username"));
    }
}
