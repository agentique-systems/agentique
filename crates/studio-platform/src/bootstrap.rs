use crate::*;
use agq_runtime_publications::{RuntimeConfig, RuntimePhase};
use std::path::{Path, PathBuf};

/// Deployment paths are explicit. Runtime discovery follows the shared package contract.
#[derive(Clone, Debug)]
pub struct NativeConfig {
    pub root: PathBuf,
    pub database: PathBuf,
    pub runtime: RuntimeConfig,
    /// Explicit self-model import into an empty custom repository. The native
    /// real-acceptance launch gate validates its isolated paths before enabling
    /// this. Ordinary project opens leave it false; semantic gates are unchanged.
    pub seed_agentique_on_empty: bool,
}

impl NativeConfig {
    pub fn for_root(root: PathBuf, runtime_dir: Option<PathBuf>) -> Result<Self> {
        let directory = agq_runtime_publications::runtime_directory(runtime_dir.as_deref())?;
        Ok(Self {
            root,
            database: directory.join("projects/agentique.sqlite"),
            runtime: RuntimeConfig {
                runtime_dir,
                ..RuntimeConfig::default()
            },
            seed_agentique_on_empty: false,
        })
    }
}

/// Observable work phases, not estimated percentages or semantic acceptance claims.
#[derive(Clone, Debug, Serialize)]
pub enum BootstrapPhase {
    Runtime(RuntimePhase),
    OpeningRepository,
    OpeningProject,
    ValidatingArchitecture,
    ValidatingAgentFabric,
    RestoringRevision,
    Ready,
}

/// Setup can be rendered before any canonical model or service exists.
#[derive(Clone, Debug, Serialize)]
pub struct SetupSurface {
    pub runtime_directory: PathBuf,
    pub database: PathBuf,
    pub kerml_profile: String,
    pub systems_profile: String,
    pub bundle_identity: String,
    pub bundle_located: bool,
    pub reason: Option<String>,
}

/// Discovery is deliberately distinct from authentication and never grants authority.
pub fn setup_surface(config: &NativeConfig) -> Result<SetupSurface> {
    let (kerml, systems) = agq_runtime_publications::accepted_contracts()?;
    let discovery = agq_runtime_publications::discover(&config.runtime);
    // `discover` returns cache locations for directories; `load` also accepts
    // archive input. Existence is UI setup information, never authentication.
    let archive_present = config
        .runtime
        .bundle
        .as_ref()
        .is_some_and(|path| path.is_file());
    Ok(SetupSurface {
        runtime_directory: agq_runtime_publications::runtime_directory(
            config.runtime.runtime_dir.as_deref(),
        )?,
        database: config.database.clone(),
        kerml_profile: kerml.profile,
        systems_profile: systems.profile,
        bundle_identity: agq_runtime_publications::accepted_bundle_identity()?,
        bundle_located: archive_present || discovery.is_ok(),
        reason: if archive_present {
            None
        } else {
            discovery.err().map(|e| e.to_string())
        },
    })
}

/// Authenticate, seed only a first-run project, and restore all default revisions.
/// Call on a worker; no HTTP, runtime download, or producer publication is involved.
pub fn open(
    config: &NativeConfig,
    mut progress: impl FnMut(BootstrapPhase),
) -> Result<StudioPlatform> {
    let runtime = agq_runtime_publications::load(&config.runtime, &config.root, |phase| {
        progress(BootstrapPhase::Runtime(phase))
    })?;
    open_authenticated(config, runtime, &mut progress)
}

/// Explicit operator setup delegates staging, authentication and atomic promotion to
/// runtime-publications. Authenticated graphs are reused without a second restoration.
pub fn install(
    config: &NativeConfig,
    bundle: &Path,
    mut progress: impl FnMut(BootstrapPhase),
) -> Result<StudioPlatform> {
    if !bundle.is_absolute() {
        return Err(PlatformError::Invalid(
            "Choose an absolute local runtime bundle path".into(),
        ));
    }
    let directory =
        agq_runtime_publications::runtime_directory(config.runtime.runtime_dir.as_deref())?;
    let installed =
        agq_runtime_publications::install_bundle(bundle, &directory, &config.root, |phase| {
            progress(BootstrapPhase::Runtime(phase))
        })?;
    open_authenticated(config, installed.runtime, &mut progress)
}

fn open_authenticated(
    config: &NativeConfig,
    runtime: agq_runtime_publications::AuthenticatedRuntime,
    progress: &mut impl FnMut(BootstrapPhase),
) -> Result<StudioPlatform> {
    progress(BootstrapPhase::OpeningRepository);
    if let Some(parent) = config
        .database
        .parent()
        .filter(|path| !path.as_os_str().is_empty())
    {
        std::fs::create_dir_all(parent).map_err(|e| PlatformError::Invalid(e.to_string()))?;
    }
    let repository = Arc::new(agq_modeling_sqlite::SqliteRepository::open(
        &config.database,
    )?);
    let service = Arc::new(ModelingService::new(repository, runtime.systems, 8));
    progress(BootstrapPhase::OpeningProject);
    let default_database =
        agq_runtime_publications::runtime_directory(config.runtime.runtime_dir.as_deref())?
            .join("projects/agentique.sqlite");
    let projects = service.repository().list_projects()?;
    if should_seed_agentique(
        &config.database,
        &default_database,
        config.seed_agentique_on_empty,
        projects.iter().map(|project| project.name.as_str()),
    ) {
        // This database contains bootstrap bookkeeping only. It never stores semantic truth.
        let journal = rusqlite::Connection::open(config.database.with_extension("views.sqlite"))
            .map_err(|e| PlatformError::Invalid(e.to_string()))?;
        journal
            .execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;")
            .map_err(|e| PlatformError::Invalid(e.to_string()))?;
        seed_agentique(&service, &config.root, &journal, &mut |phase| match phase {
            "validating_architecture" => progress(BootstrapPhase::ValidatingArchitecture),
            "validating_agent_fabric" => progress(BootstrapPhase::ValidatingAgentFabric),
            _ => {}
        })?;
    }
    progress(BootstrapPhase::RestoringRevision);
    for project in service.repository().list_projects()? {
        let branch = service
            .repository()
            .get_branch(project.id, project.default_branch)?;
        service.resolve(project.id, RevisionSelector::Revision(branch.head))?;
    }
    progress(BootstrapPhase::Ready);
    Ok(StudioPlatform::new(service, AgentPolicy::operator()))
}

fn should_seed_agentique<'a>(
    database: &Path,
    default_database: &Path,
    explicitly_seed_empty: bool,
    project_names: impl Iterator<Item = &'a str>,
) -> bool {
    let mut empty = true;
    for name in project_names {
        empty = false;
        if name == "Agentique" {
            // The seed operation authenticates its persisted plan or exact
            // untouched bootstrap metadata before it may resume any work.
            return true;
        }
    }
    empty && (database == default_database || explicitly_seed_empty)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generic_restart_never_creates_sample_but_pending_seed_can_resume() {
        let default = Path::new("runtime/projects/agentique.sqlite");
        let generic = Path::new("user-projects/engine.sqlite");
        // Ordinary first launch seeds only the default repository. A separately
        // authorized self-model import can seed an empty isolated repository.
        assert!(should_seed_agentique(
            default,
            default,
            false,
            [].into_iter()
        ));
        assert!(!should_seed_agentique(
            generic,
            default,
            false,
            [].into_iter()
        ));
        assert!(should_seed_agentique(
            generic,
            default,
            true,
            [].into_iter()
        ));
        // New Project persists an initially empty Working project. On restart
        // that authored project must remain the only project at either path.
        for path in [default, generic] {
            assert!(!should_seed_agentique(
                path,
                default,
                false,
                ["Engine"].into_iter()
            ));
            assert!(!should_seed_agentique(
                path,
                default,
                false,
                ["Engine", "Sensor"].into_iter()
            ));
            // Existing bootstrap journals also remain resumable in an explicit
            // acceptance repository or after creating another project beside it.
            assert!(should_seed_agentique(
                path,
                default,
                false,
                ["Engine", "Agentique"].into_iter()
            ));
        }
        assert!(
            !should_seed_agentique(generic, default, true, ["Engine"].into_iter()),
            "Explicit empty-repository import must not replace an existing generic project"
        );
    }

    #[test]
    fn discovery_is_not_authentication_and_missing_runtime_never_creates_a_database() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = NativeConfig::for_root(
            directory.path().into(),
            Some(directory.path().join("runtime")),
        )
        .unwrap();
        // Explicit missing bundle makes the test independent of legacy environment inputs.
        config.runtime.bundle = Some(directory.path().join("absent.agq-runtime"));
        config.seed_agentique_on_empty = true;
        assert!(open(&config, |_| {}).is_err());
        assert!(!config.database.exists());
        let setup = setup_surface(&config).unwrap();
        assert!(!setup.bundle_located);
        assert!(setup.reason.is_some());
        assert_eq!(setup.kerml_profile, "agentique-kerml-1.0-operational/9");
        assert_eq!(setup.systems_profile, "agentique-sysml-2.0-operational/3");
    }

    #[test]
    fn relative_setup_path_is_rejected_before_installation() {
        let directory = tempfile::tempdir().unwrap();
        let config = NativeConfig::for_root(
            directory.path().into(),
            Some(directory.path().join("runtime")),
        )
        .unwrap();
        assert!(install(&config, Path::new("runtime.agq-runtime"), |_| {}).is_err());
        assert!(!config.database.exists());
    }

    #[test]
    fn existing_archive_does_not_claim_authenticated_runtime() {
        let directory = tempfile::tempdir().unwrap();
        let mut config = NativeConfig::for_root(
            directory.path().into(),
            Some(directory.path().join("runtime")),
        )
        .unwrap();
        let bundle = directory.path().join("invalid.agq-runtime");
        std::fs::write(&bundle, b"untrusted bytes").unwrap();
        config.runtime.bundle = Some(bundle);
        assert!(setup_surface(&config).unwrap().bundle_located);
        assert!(open(&config, |_| {}).is_err());
        assert!(!config.database.exists());
    }
}
