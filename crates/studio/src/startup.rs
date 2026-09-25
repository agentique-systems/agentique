//! Runtime setup has its own observable state, available before semantic services open.
use super::*;
use agq_runtime_publications::{RuntimeConfig, RuntimePhase};
use std::{path::Path, time::Instant};

#[derive(Clone, Serialize)]
struct PhaseTiming {
    phase: String,
    duration_ms: u128,
}

pub(super) struct Startup {
    phase: String,
    running: bool,
    kerml: &'static str,
    sysml: &'static str,
    error: Option<String>,
    phases: Vec<PhaseTiming>,
    started: Instant,
    phase_started: Instant,
}
impl Startup {
    pub(super) fn new() -> Self {
        Self {
            phase: "locating_package".into(),
            running: true,
            kerml: "Not authenticated",
            sysml: "Not authenticated",
            error: None,
            phases: Vec::new(),
            started: Instant::now(),
            phase_started: Instant::now(),
        }
    }
    fn advance(&mut self, phase: &str) {
        if self.phase == phase {
            return;
        }
        let elapsed = self.phase_started.elapsed().as_millis();
        self.phases.push(PhaseTiming {
            phase: self.phase.clone(),
            duration_ms: elapsed,
        });
        eprintln!(
            "{}",
            json!({"studio_phase":self.phase,"duration_ms":elapsed})
        );
        self.phase = phase.into();
        self.phase_started = Instant::now();
        match phase {
            "authenticating_kerml" => self.kerml = "Authenticating",
            "authenticating_sysml" => {
                self.kerml = "Authenticated";
                self.sysml = "Authenticating";
            }
            "publications_authenticated" | "installing" => {
                self.kerml = "Authenticated";
                self.sysml = "Authenticated";
            }
            _ => {}
        }
    }
    fn value(&self) -> Value {
        json!({
            "phase":self.phase,"running":self.running,"ready":self.phase == "ready",
            "publications":[
                {"name":"KerML Operational v9","status":self.kerml},
                {"name":"SysML Operational v3","status":self.sysml}
            ],
            "error":self.error,"phases":self.phases,
            "elapsed_ms":if self.running { self.started.elapsed().as_millis() } else { self.phases.iter().map(|phase| phase.duration_ms).sum::<u128>() },
            "current_phase_ms":if self.running { self.phase_started.elapsed().as_millis() } else { 0 }
        })
    }
}

pub(super) fn runtime_config(config: &StudioConfig) -> RuntimeConfig {
    RuntimeConfig {
        runtime_dir: config.runtime_dir.clone(),
        bundle: config.bundle.clone(),
        kerml_cache: config.kerml_cache.clone(),
        systems_cache: config.systems_cache.clone(),
    }
}
pub(super) fn phase_name(phase: RuntimePhase) -> &'static str {
    match phase {
        RuntimePhase::LocatingPackage => "locating_package",
        RuntimePhase::Copying => "copying",
        RuntimePhase::AuthenticatingKerml => "authenticating_kerml",
        RuntimePhase::AuthenticatingSysml => "authenticating_sysml",
        RuntimePhase::Installing => "installing",
        RuntimePhase::Ready => "publications_authenticated",
    }
}

pub(super) fn start(host: Host, bundle: Option<PathBuf>) {
    std::thread::spawn(move || {
        let mut progress = |phase: &str| host.startup.lock().expect("startup state").advance(phase);
        let result = (|| {
            if host.operator_token.len() < 16
                || host.agent_token.len() < 16
                || host.operator_token == host.agent_token
            {
                return Err(
                    "Studio operator and agent tokens must be distinct and at least 16 characters."
                        .into(),
                );
            }
            if let Some(bundle) = bundle {
                let directory =
                    agq_runtime_publications::runtime_directory(host.config.runtime_dir.as_deref())
                        .map_err(|error| error.to_string())?;
                let installed = agq_runtime_publications::install_bundle(
                    &bundle,
                    &directory,
                    &host.config.root,
                    |phase| progress(phase_name(phase)),
                )
                .map_err(|error| error.to_string())?;
                return initialize_with_publications(
                    &host.config,
                    &mut progress,
                    installed.runtime,
                )
                .map(Arc::new);
            }
            initialize(&host.config, &mut progress).map(Arc::new)
        })();
        let mut state = host.startup.lock().expect("startup state");
        match &result {
            Ok(_) => state.advance("ready"),
            Err(error) => {
                if state.kerml == "Authenticating" {
                    state.kerml = "Authentication failed";
                }
                if state.sysml == "Authenticating" {
                    state.sysml = "Authentication failed";
                }
                state.advance("setup_required");
                state.error = Some(error.clone());
                eprintln!("Studio setup required: {error}");
            }
        }
        state.running = false;
        *host.runtime.lock().expect("runtime initialization") = result;
    });
}

pub(super) async fn status(State(host): State<Host>, headers: HeaderMap) -> HttpResult {
    let actor = if headers.contains_key("authorization") {
        policy(&host, &headers)?
    } else {
        AgentPolicy::operator()
    };
    let mut value = host.startup.lock().expect("startup state").value();
    value["session_token"] = actor
        .permissions
        .contains(&Authority::Commit)
        .then(|| host.operator_token.as_str())
        .into();
    value["runtime_directory"] =
        agq_runtime_publications::runtime_directory(host.config.runtime_dir.as_deref())
            .ok()
            .map(|path| path.display().to_string())
            .into();
    value["setup_command"] =
        "cargo run --locked --offline -p agq-studio -- setup --bundle <bundle>".into();
    Ok(Json(value))
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct InstallRequest {
    bundle: PathBuf,
}

fn begin(host: &Host, headers: &HeaderMap) -> Result<(), HttpError> {
    policy(host, headers)?
        .require(Authority::Commit)
        .map_err(agent_error)?;
    let mut state = host.startup.lock().expect("startup state");
    if state.running || host.runtime.lock().expect("runtime state").is_ok() {
        return Err(HttpError(
            StatusCode::CONFLICT,
            "Studio is already opening or has an authenticated runtime.".into(),
        ));
    }
    *state = Startup::new();
    Ok(())
}

pub(super) async fn install(
    State(host): State<Host>,
    headers: HeaderMap,
    Json(request): Json<InstallRequest>,
) -> HttpResult {
    if request.bundle.as_os_str().is_empty() || !Path::new(&request.bundle).is_absolute() {
        return Err(failed(
            "Choose the absolute path to an accepted runtime bundle directory or .agq-runtime file.",
        ));
    }
    begin(&host, &headers)?;
    start(host, Some(request.bundle));
    Ok(Json(json!({"started":true})))
}
pub(super) async fn retry(State(host): State<Host>, headers: HeaderMap) -> HttpResult {
    begin(&host, &headers)?;
    start(host, None);
    Ok(Json(json!({"started":true})))
}
