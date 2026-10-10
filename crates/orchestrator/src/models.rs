//! Models per role (C-54, ROADMAP §4.16 "Models per role", "Credentials"):
//! each role's model, effort and fallback as Settings configure them,
//! resolved once before an objective starts against the credentials
//! Agentique may use, and recorded with the reasons (which credential, who
//! pays, why a fallback). The Studio, which owns Settings and the keys,
//! calls [`resolve`]; the Orchestrator uses what was recorded for every
//! session of a role ([`for_role`]); [`decider`] builds typed decisions
//! from it for an exploring objective (W12.5 uses it). No role takes
//! another's model, and nothing moves to another credential while the
//! objective runs.

use crate::decide::Decider;
use crate::explore::Deciding;
use crate::record::{Access, RoleModel};
use agq_assistant::claude_agent::Login;
use agq_providers::{Credential, KeyStatus, ModelRef, Provider, capabilities};

/// How a role reaches its model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    /// A Claude Agent runtime session: Anthropic's API (with a key or the
    /// Claude subscription token) or an Anthropic-compatible endpoint.
    Session,
    /// One call at a time through the provider layer (the explorer's step,
    /// escalation): a key, never the subscription token.
    Direct,
    /// A typed decision (Jev).
    Decisions,
}

/// The roles with a model, in the order Settings and the Objectives panel
/// show them.
pub const ROLES: [(&str, Kind); 7] = [
    ("lead", Kind::Session),
    ("implementer", Kind::Session),
    ("reviewer", Kind::Session),
    ("evaluator", Kind::Session),
    ("explorer", Kind::Direct),
    ("escalation", Kind::Direct),
    ("decisions", Kind::Decisions),
];

/// Whether an objective needs `role` to start: the cycle's four sessions
/// always (the evaluator too, since any change may need evaluating); the
/// explorer, escalation and typed decisions only when it explores.
pub fn needed(role: &str, explore: bool) -> bool {
    explore || matches!(role, "lead" | "implementer" | "reviewer" | "evaluator")
}

/// What an objective's roles resolved to.
#[derive(Clone, Debug, PartialEq)]
pub struct Resolved {
    /// Each role with a model now.
    pub models: Vec<RoleModel>,
    /// Each role the objective does not need that has no model now, and
    /// why: recorded, never a reason not to start.
    pub unavailable: Vec<(String, String)>,
}

/// A model and its effort.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Choice {
    pub model: ModelRef,
    /// `None` for the model's default (or a model without efforts).
    pub effort: Option<String>,
}

/// One role's configuration in Settings.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Configured {
    pub role: String,
    pub model: Choice,
    pub fallback: Option<Choice>,
}

/// What the resolution knows about the credentials Agentique may use.
pub struct Credentials<'a> {
    /// Where a key or the subscription token comes from (the environment
    /// or the credential store, as `agq_providers` reads them).
    pub status: &'a dyn Fn(Credential) -> KeyStatus,
    /// With both an Anthropic API key and the Claude subscription token,
    /// whether sessions use the token (the default) or the key.
    pub prefer_subscription: bool,
    /// Whether this computer has a Claude login, when it was checked: it is
    /// never used, and a fallback from Anthropic says so.
    pub login: Option<&'a Login>,
}

/// What a role's model runs on.
struct Found {
    access: Access,
    credential: String,
    billed: String,
}

/// Resolves every configured role: its own model when its provider has a
/// credential Agentique may use for that kind of role, else its fallback
/// with the reason, else none. A role the objective needs ([`needed`]) with
/// none is named with what is missing and nothing starts; another is
/// recorded as unavailable.
pub fn resolve(
    configured: &[Configured],
    credentials: &Credentials,
    explore: bool,
) -> Result<Resolved, Vec<String>> {
    let mut resolved = Resolved {
        models: Vec::new(),
        unavailable: Vec::new(),
    };
    let mut problems = Vec::new();
    for (role, outcome) in resolve_each(configured, credentials) {
        match outcome {
            Ok(model) => resolved.models.push(model),
            Err(problem) if needed(role, explore) => problems.push(problem),
            Err(problem) => resolved.unavailable.push((role.to_string(), problem)),
        }
    }
    if problems.is_empty() {
        Ok(resolved)
    } else {
        Err(problems)
    }
}

/// [`resolve`], role by role, for Settings to show each one's outcome.
pub fn resolve_each(
    configured: &[Configured],
    credentials: &Credentials,
) -> Vec<(&'static str, Result<RoleModel, String>)> {
    ROLES
        .iter()
        .map(|(role, kind)| (*role, resolve_one(role, *kind, configured, credentials)))
        .collect()
}

fn resolve_one(
    role: &str,
    kind: Kind,
    configured: &[Configured],
    credentials: &Credentials,
) -> Result<RoleModel, String> {
    let setting = configured
        .iter()
        .find(|c| c.role == role)
        .ok_or_else(|| format!("{role}: no model is set in Settings › Agents"))?;
    let (choice, found, fallback) = match find(role, kind, &setting.model.model, credentials) {
        Ok(found) => (&setting.model, found, None),
        Err(reason) => match &setting.fallback {
            Some(fallback) => match find(role, kind, &fallback.model, credentials) {
                Ok(found) => (fallback, found, Some(reason)),
                Err(again) => {
                    return Err(format!(
                        "{role}: {} {reason}; its fallback {} {again}",
                        setting.model.model, fallback.model
                    ));
                }
            },
            None => {
                return Err(format!(
                    "{role}: {} {reason}, and no fallback is set (Settings › Agents)",
                    setting.model.model
                ));
            }
        },
    };
    Ok(RoleModel {
        role: role.to_string(),
        model: choice.model.clone(),
        effort: agq_assistant::ModelChoice::new(choice.model.clone(), choice.effort.clone()).effort,
        access: found.access,
        configured: setting.model.model.clone(),
        fallback,
        credential: found.credential,
        billed: found.billed,
    })
}

/// The credential `model` would run on for a role of `kind`, or why there
/// is none (worded to follow the model's name).
fn find(
    role: &str,
    kind: Kind,
    model: &ModelRef,
    credentials: &Credentials,
) -> Result<Found, String> {
    let capable = capabilities(model);
    match kind {
        Kind::Session if !(capable.chat && capable.tools && capable.agent_runtime) => {
            return Err(
                "cannot run a Claude Agent runtime session (it reaches Anthropic's and DeepSeek's models only)"
                    .into(),
            );
        }
        Kind::Direct if !capable.chat => return Err("does not answer a conversation".into()),
        Kind::Decisions if !capable.decisions => {
            return Err("is not a known typed-decision model".into());
        }
        _ => {}
    }
    // Where each credential comes from, or why it cannot be used: a store
    // that cannot be read is one candidate's problem, never the others'
    // (an environment variable still counts without it).
    let available = |credential: Credential| match (credentials.status)(credential) {
        KeyStatus::FromEnvironment { variable } => Ok(Some(variable.to_string())),
        KeyStatus::Stored => Ok(Some("the Windows Credential Manager".to_string())),
        KeyStatus::Missing => Ok(None),
        KeyStatus::Unavailable(why) => Err(format!(
            "cannot be used: the credential store cannot be read ({why})"
        )),
    };
    let key = Credential::Key(model.provider);
    if model.provider != Provider::Anthropic {
        return match available(key)? {
            Some(source) => Ok(Found {
                access: Access::Key,
                credential: source,
                billed: format!(
                    "per token, to the {} account of this key",
                    model.provider.name()
                ),
            }),
            None => Err(format!(
                "needs a {} key (Settings › Providers › {0}, or {})",
                model.provider.name(),
                model.provider.key_variable()
            )),
        };
    }
    let with_key = |source: String| Found {
        access: Access::Key,
        credential: source,
        billed: "per token, to the Anthropic Console account of this key".into(),
    };
    let with_token = |source: String| Found {
        access: Access::Subscription,
        credential: format!("the Claude subscription token ({source})"),
        billed: "your Claude plan, within its usage limits (spend is shown at API prices)".into(),
    };
    let api_key = available(key);
    let token = available(Credential::ClaudeSubscription);
    let usable = |found: &Result<Option<String>, String>| found.as_ref().ok().cloned().flatten();
    // Neither can be used: an unreadable store is the reason when there is
    // one, else what is missing.
    let unreadable = [&api_key, &token]
        .into_iter()
        .find_map(|found| found.as_ref().err().cloned());
    if kind == Kind::Session {
        let (first, second) = if credentials.prefer_subscription {
            (
                usable(&token).map(with_token),
                usable(&api_key).map(with_key),
            )
        } else {
            (
                usable(&api_key).map(with_key),
                usable(&token).map(with_token),
            )
        };
        if let Some(found) = first.or(second) {
            return Ok(found);
        }
        return Err(unreadable.unwrap_or_else(|| {
            format!(
                "needs an Anthropic API key or a Claude subscription token (Settings › Providers › Anthropic){}",
                login_note(credentials.login)
            )
        }));
    }
    match (usable(&api_key), usable(&token)) {
        (Some(source), _) => Ok(with_key(source)),
        (None, Some(_)) => Err(format!(
            "needs an Anthropic API key: the {role} role calls its model directly, and a Claude subscription token works only in the Claude Agent runtime"
        )),
        (None, None) => Err(api_key.err().unwrap_or_else(|| {
            format!(
                "needs an Anthropic API key (Settings › Providers › Anthropic, or ANTHROPIC_API_KEY){}",
                login_note(credentials.login)
            )
        })),
    }
}

/// Why this computer's own Claude login does not count (C-54).
fn login_note(login: Option<&Login>) -> String {
    const WHY: &str = "Anthropic does not allow products built on the Claude Agent SDK to offer claude.ai login without its approval";
    match login.map(Login::describe) {
        Some(Some(login)) => {
            format!("; the Claude login on this computer ({login}) is not used: {WHY}")
        }
        Some(None) => String::new(),
        None => format!("; a Claude login on this computer, if there is one, is not used: {WHY}"),
    }
}

/// The recorded model of `role`, for every session of it.
pub fn for_role<'a>(models: &'a [RoleModel], role: &str) -> Result<&'a RoleModel, String> {
    models.iter().find(|m| m.role == role).ok_or_else(|| {
        format!(
            "no model was resolved for the {role} role when this objective started; stop it and start it again"
        )
    })
}

/// The typed decisions of the objective (C-54): Jev's model from the
/// `decisions` role, and the reasoning model it escalates to (with its
/// effort) from the `escalation` role, instead of `Decider::default`.
pub fn decider(models: &[RoleModel]) -> Result<Decider, String> {
    let decisions = for_role(models, "decisions")?;
    let escalation = for_role(models, "escalation")?;
    Ok(Decider {
        jev_model: decisions.model.model.clone(),
        model: escalation.model.clone(),
        effort: escalation.effort.clone(),
        ..Decider::default()
    })
}

/// What an exploring objective's run decides with (W12.4's [`Deciding`]),
/// from its recorded roles in one call: Jev through the [`decider`] of its
/// `decisions` and `escalation` roles, and the explorer's model and effort
/// from its `explorer` role, to which Jev escalates a step it is unsure of
/// (the W13.7 repair), and the escalation role's model and effort, which
/// decide the steps that test a hypothesis (E3); the decider takes it too,
/// for the typed decisions that escalate, such as reading an objective's
/// intent. `run` gets them for the run's length.
pub fn with_deciding<T>(
    models: &[RoleModel],
    stand_in: Option<&dyn crate::decide::Answers>,
    run: impl FnOnce(&Deciding) -> T,
) -> Result<T, String> {
    let decider = decider(models)?;
    let explorer = for_role(models, "explorer")?;
    let escalation = for_role(models, "escalation")?;
    let deciding = Deciding {
        answers: stand_in.unwrap_or(&decider),
        explorer: explorer.model.clone(),
        effort: explorer.effort.clone(),
        escalation: escalation.model.clone(),
        escalation_effort: escalation.effort.clone(),
    };
    Ok(run(&deciding))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn choice(text: &str, effort: Option<&str>) -> Choice {
        Choice {
            model: ModelRef::parse(text).unwrap(),
            effort: effort.map(str::to_string),
        }
    }

    /// The defaults of Settings › Agents (C-54).
    fn defaults() -> Vec<Configured> {
        let pro = |effort| Some(choice("deepseek/deepseek-v4-pro", Some(effort)));
        let role = |role: &str, model: Choice, fallback: Option<Choice>| Configured {
            role: role.into(),
            model,
            fallback,
        };
        vec![
            role(
                "lead",
                choice("anthropic/claude-opus-5-5", Some("high")),
                pro("max"),
            ),
            role(
                "implementer",
                choice("anthropic/claude-sonnet-5-5", Some("high")),
                pro("high"),
            ),
            role(
                "reviewer",
                choice("anthropic/claude-opus-5-5", Some("high")),
                pro("max"),
            ),
            role(
                "evaluator",
                choice("anthropic/claude-sonnet-5-5", Some("medium")),
                pro("high"),
            ),
            role(
                "explorer",
                choice("deepseek/deepseek-flash", Some("high")),
                None,
            ),
            role(
                "escalation",
                choice("anthropic/claude-opus-5-5", Some("high")),
                pro("max"),
            ),
            role("decisions", choice("typesafe/jev-1.13.0", None), None),
        ]
    }

    fn with(present: &'static [Credential]) -> impl Fn(Credential) -> KeyStatus {
        move |credential| {
            if present.contains(&credential) {
                KeyStatus::FromEnvironment {
                    variable: credential.variable(),
                }
            } else {
                KeyStatus::Missing
            }
        }
    }

    const DEEPSEEK: Credential = Credential::Key(Provider::DeepSeek);
    const JEV: Credential = Credential::Key(Provider::TypeSafe);
    const ANTHROPIC: Credential = Credential::Key(Provider::Anthropic);
    const TOKEN: Credential = Credential::ClaudeSubscription;

    fn model<'a>(models: &'a [RoleModel], role: &str) -> &'a RoleModel {
        for_role(models, role).unwrap()
    }

    #[test]
    fn each_role_runs_on_its_own_model_when_its_credential_is_there() {
        let status = with(&[ANTHROPIC, DEEPSEEK, JEV]);
        let credentials = Credentials {
            status: &status,
            prefer_subscription: true,
            login: None,
        };
        let models = resolve(&defaults(), &credentials, true).unwrap().models;
        assert_eq!(models.len(), 7);
        let lead = model(&models, "lead");
        assert_eq!(lead.model.to_string(), "anthropic/claude-opus-5-5");
        assert_eq!(lead.effort.as_deref(), Some("high"));
        assert_eq!((lead.access, lead.fallback.as_deref()), (Access::Key, None));
        assert_eq!(lead.credential, "ANTHROPIC_API_KEY");
        assert!(lead.billed.contains("Console account"), "{}", lead.billed);
        assert_eq!(
            model(&models, "implementer").model.model,
            "claude-sonnet-5-5"
        );
        assert_eq!(
            model(&models, "evaluator").effort.as_deref(),
            Some("medium")
        );
        assert_eq!(model(&models, "explorer").model.model, "deepseek-flash");
        let decisions = model(&models, "decisions");
        assert_eq!(
            (decisions.model.model.as_str(), decisions.effort.as_deref()),
            ("jev-1.13.0", None)
        );
        assert_eq!(model(&models, "escalation").access, Access::Key);
        // Typed decisions take their models from these roles.
        let decider = decider(&models).unwrap();
        assert_eq!(decider.jev_model, "jev-1.13.0");
        assert_eq!(decider.model.to_string(), "anthropic/claude-opus-5-5");
        assert_eq!(decider.effort.as_deref(), Some("high"));
        // Exploration's deciding, from the explorer and decisions roles in
        // one call (W12.5 passes the record's models): Jev escalates a step
        // to the explorer's model (the W13.7 repair).
        let seen = with_deciding(&models, None, |deciding| {
            (
                deciding.answers.jev_model().to_string(),
                deciding.explorer.to_string(),
                deciding.effort.clone(),
            )
        })
        .unwrap();
        assert_eq!(
            seen,
            (
                "typesafe/jev-1.13.0".to_string(),
                "deepseek/deepseek-flash".to_string(),
                Some("high".to_string()),
            )
        );
        // A record without the exploring roles cannot explore.
        let sessions: Vec<RoleModel> = models
            .iter()
            .filter(|m| needed(&m.role, false))
            .cloned()
            .collect();
        assert!(
            with_deciding(&sessions, None, |_| ())
                .unwrap_err()
                .contains("no model was resolved")
        );
    }

    /// The reference machine (C-54, 2026-10-04): the Operator's Claude
    /// subscription token, DeepSeek and TypeSafe AI keys, no Anthropic API
    /// key. Sessions run on the token; escalation, which calls its model
    /// directly, falls back to DeepSeek with the reason.
    #[test]
    fn with_the_subscription_token_sessions_use_it_and_direct_calls_fall_back() {
        let status = with(&[TOKEN, DEEPSEEK, JEV]);
        let credentials = Credentials {
            status: &status,
            prefer_subscription: true,
            login: None,
        };
        let models = resolve(&defaults(), &credentials, true).unwrap().models;
        for role in ["lead", "implementer", "reviewer", "evaluator"] {
            let m = model(&models, role);
            assert_eq!(m.model.provider, Provider::Anthropic, "{role}");
            assert_eq!(m.access, Access::Subscription, "{role}");
            assert_eq!(m.fallback, None, "{role}");
            assert!(
                m.credential.contains("CLAUDE_CODE_OAUTH_TOKEN"),
                "{}",
                m.credential
            );
            assert!(m.billed.contains("Claude plan"), "{}", m.billed);
        }
        assert_eq!(
            model(&models, "lead").label(),
            "claude-opus-5-5 · high · Claude subscription (your plan's limits apply)"
        );
        let escalation = model(&models, "escalation");
        assert_eq!(escalation.model.to_string(), "deepseek/deepseek-v4-pro");
        assert_eq!(escalation.effort.as_deref(), Some("max"));
        assert_eq!(
            escalation.configured.to_string(),
            "anthropic/claude-opus-5-5"
        );
        let why = escalation.fallback.as_deref().unwrap();
        assert!(
            why.contains("works only in the Claude Agent runtime"),
            "{why}"
        );
        assert_eq!(escalation.access, Access::Key);
        // With both, a setting chooses; the key when asked.
        let both = with(&[TOKEN, ANTHROPIC, DEEPSEEK, JEV]);
        let credentials = Credentials {
            status: &both,
            prefer_subscription: false,
            login: None,
        };
        let models = resolve(&defaults(), &credentials, true).unwrap().models;
        assert_eq!(model(&models, "lead").access, Access::Key);
        let credentials = Credentials {
            status: &both,
            prefer_subscription: true,
            login: None,
        };
        let models = resolve(&defaults(), &credentials, true).unwrap().models;
        assert_eq!(model(&models, "lead").access, Access::Subscription);
        assert_eq!(model(&models, "escalation").access, Access::Key);
    }

    /// Without any Anthropic credential each Claude role falls back with the
    /// reason, and a claude.ai login on this computer is named and not used.
    #[test]
    fn without_anthropic_credentials_roles_fall_back_and_say_why() {
        let status = with(&[DEEPSEEK, JEV]);
        let login = Login {
            logged_in: true,
            method: "claude.ai".into(),
            api_provider: Some("firstParty".into()),
            subscription: Some("max".into()),
        };
        let credentials = Credentials {
            status: &status,
            prefer_subscription: true,
            login: Some(&login),
        };
        let models = resolve(&defaults(), &credentials, true).unwrap().models;
        let lead = model(&models, "lead");
        assert_eq!(lead.model.to_string(), "deepseek/deepseek-v4-pro");
        assert_eq!(lead.effort.as_deref(), Some("max"));
        let why = lead.fallback.as_deref().unwrap();
        assert!(
            why.contains("needs an Anthropic API key or a Claude subscription token")
                && why.contains("claude.ai (Max)")
                && why.contains("not used")
                && why.contains("without its approval"),
            "{why}"
        );
        assert!(lead.billed.contains("DeepSeek account"));
        assert_eq!(
            model(&models, "implementer").effort.as_deref(),
            Some("high")
        );
        // The explorer's own model is there: no fallback.
        assert_eq!(model(&models, "explorer").fallback, None);
    }

    /// The review of W12.3: an objective that does not explore needs only
    /// the cycle's four sessions; an Operator with only an Anthropic key
    /// starts one, and the roles without a model are recorded as such.
    #[test]
    fn an_objective_that_does_not_explore_needs_only_its_sessions() {
        let status = with(&[ANTHROPIC]);
        let credentials = Credentials {
            status: &status,
            prefer_subscription: true,
            login: None,
        };
        let resolved = resolve(&defaults(), &credentials, false).unwrap();
        let roles: Vec<&str> = resolved.models.iter().map(|m| m.role.as_str()).collect();
        // Escalation runs on Opus 5.5 with the key; the explorer and typed
        // decisions have no credential, which this objective does not need.
        assert_eq!(
            roles,
            ["lead", "implementer", "reviewer", "evaluator", "escalation"]
        );
        let unavailable: Vec<&str> = resolved
            .unavailable
            .iter()
            .map(|(r, _)| r.as_str())
            .collect();
        assert_eq!(unavailable, ["explorer", "decisions"]);
        assert!(resolved.unavailable[1].1.contains("TypeSafe AI key"));
        // An exploring objective needs them all.
        let problems = resolve(&defaults(), &credentials, true).unwrap_err();
        assert_eq!(problems.len(), 2, "{problems:?}");
        assert!(
            needed("evaluator", false) && !needed("explorer", false) && needed("explorer", true)
        );
    }

    /// The review of W12.3: a credential store that cannot be read (no
    /// store, or a locked one) is one candidate's problem: a key in the
    /// environment still counts.
    #[test]
    fn an_unreadable_store_does_not_hide_a_key_in_the_environment() {
        let status = |credential: Credential| match credential {
            Credential::Key(Provider::Anthropic) => KeyStatus::FromEnvironment {
                variable: "ANTHROPIC_API_KEY",
            },
            _ => KeyStatus::Unavailable("no credential store".into()),
        };
        let credentials = Credentials {
            status: &status,
            prefer_subscription: true,
            login: None,
        };
        let resolved = resolve(&defaults(), &credentials, false).unwrap();
        let lead = model(&resolved.models, "lead");
        assert_eq!((lead.access, lead.fallback.as_deref()), (Access::Key, None));
        assert_eq!(lead.credential, "ANTHROPIC_API_KEY");
        assert_eq!(model(&resolved.models, "escalation").access, Access::Key);
        // Nothing usable: the unreadable store is named.
        let nothing = |_: Credential| KeyStatus::Unavailable("no credential store".into());
        let credentials = Credentials {
            status: &nothing,
            prefer_subscription: true,
            login: None,
        };
        let problems = resolve(&defaults(), &credentials, false).unwrap_err();
        assert!(
            problems[0].contains("cannot be read (no credential store)"),
            "{problems:?}"
        );
    }

    /// A role with neither its model nor its fallback stops the objective
    /// from starting, naming the role and what is missing.
    #[test]
    fn a_role_with_neither_is_named_and_nothing_starts() {
        let status = with(&[DEEPSEEK]);
        let credentials = Credentials {
            status: &status,
            prefer_subscription: true,
            login: None,
        };
        let problems = resolve(&defaults(), &credentials, true).unwrap_err();
        assert_eq!(problems.len(), 1, "{problems:?}");
        assert!(
            problems[0].starts_with("decisions: typesafe/jev-1.13.0 needs a TypeSafe AI key")
                && problems[0].contains("no fallback is set"),
            "{problems:?}"
        );
        let nothing = with(&[]);
        let credentials = Credentials {
            status: &nothing,
            prefer_subscription: true,
            login: None,
        };
        let problems = resolve(&defaults(), &credentials, true).unwrap_err();
        assert_eq!(problems.len(), 7, "{problems:?}");
        assert!(problems[0].contains("its fallback deepseek/deepseek-v4-pro needs a DeepSeek key"));
        // A model that cannot do the role's work is not used for it.
        let mut configured = defaults();
        configured[0].model = choice("typesafe/jev-1.13.0", None);
        configured[0].fallback = None;
        let all = with(&[ANTHROPIC, DEEPSEEK, JEV]);
        let credentials = Credentials {
            status: &all,
            prefer_subscription: true,
            login: None,
        };
        let problems = resolve(&configured, &credentials, true).unwrap_err();
        assert!(
            problems[0].contains("cannot run a Claude Agent runtime session"),
            "{problems:?}"
        );
        assert!(
            for_role(&[], "lead")
                .unwrap_err()
                .contains("start it again")
        );
    }
}
