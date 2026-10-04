//! Models per role (C-54, ROADMAP §4.16 "Models per role", "Credentials"):
//! each role's model, effort and fallback as Settings configure them,
//! resolved once before an objective starts against the credentials
//! Agentique may use, and recorded with the reasons (which credential, who
//! pays, why a fallback). The Studio, which owns Settings and the keys,
//! calls [`resolve`]; the Orchestrator uses what was recorded for every
//! session of a role ([`for_role`]) and for typed decisions ([`decider`]).
//! No role takes another's model, and nothing moves to another credential
//! while the objective runs.

use crate::decide::Decider;
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
/// with the reason, else the role is named with what is missing and nothing
/// starts. Every role of [`ROLES`] must be configured.
pub fn resolve(
    configured: &[Configured],
    credentials: &Credentials,
) -> Result<Vec<RoleModel>, Vec<String>> {
    let mut models = Vec::new();
    let mut problems = Vec::new();
    for (role, kind) in ROLES {
        let Some(setting) = configured.iter().find(|c| c.role == role) else {
            problems.push(format!("{role}: no model is set in Settings › Agents"));
            continue;
        };
        let own = find(role, kind, &setting.model.model, credentials);
        let found = match own {
            Ok(found) => Ok((&setting.model, found, None)),
            Err(reason) => match &setting.fallback {
                Some(fallback) => match find(role, kind, &fallback.model, credentials) {
                    Ok(found) => Ok((fallback, found, Some(reason))),
                    Err(again) => Err(format!(
                        "{role}: {} {reason}; its fallback {} {again}",
                        setting.model.model, fallback.model
                    )),
                },
                None => Err(format!(
                    "{role}: {} {reason}, and no fallback is set (Settings › Agents)",
                    setting.model.model
                )),
            },
        };
        match found {
            Ok((choice, found, fallback)) => models.push(RoleModel {
                role: role.to_string(),
                model: choice.model.clone(),
                effort: agq_assistant::ModelChoice::new(
                    choice.model.clone(),
                    choice.effort.clone(),
                )
                .effort,
                access: found.access,
                configured: setting.model.model.clone(),
                fallback,
                credential: found.credential,
                billed: found.billed,
            }),
            Err(problem) => problems.push(problem),
        }
    }
    if problems.is_empty() {
        Ok(models)
    } else {
        Err(problems)
    }
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
    let api_key = available(key)?;
    let token = available(Credential::ClaudeSubscription)?;
    if kind == Kind::Session {
        let (first, second) = if credentials.prefer_subscription {
            (token.clone().map(with_token), api_key.map(with_key))
        } else {
            (api_key.map(with_key), token.clone().map(with_token))
        };
        if let Some(found) = first.or(second) {
            return Ok(found);
        }
        return Err(format!(
            "needs an Anthropic API key or a Claude subscription token (Settings › Providers › Anthropic){}",
            login_note(credentials.login)
        ));
    }
    match (api_key, token) {
        (Some(source), _) => Ok(with_key(source)),
        (None, Some(_)) => Err(format!(
            "needs an Anthropic API key: the {role} role calls its model directly, and a Claude subscription token works only in the Claude Agent runtime"
        )),
        (None, None) => Err(format!(
            "needs an Anthropic API key (Settings › Providers › Anthropic, or ANTHROPIC_API_KEY){}",
            login_note(credentials.login)
        )),
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
        let models = resolve(&defaults(), &credentials).unwrap();
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
        let models = resolve(&defaults(), &credentials).unwrap();
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
        let models = resolve(&defaults(), &credentials).unwrap();
        assert_eq!(model(&models, "lead").access, Access::Key);
        let credentials = Credentials {
            status: &both,
            prefer_subscription: true,
            login: None,
        };
        let models = resolve(&defaults(), &credentials).unwrap();
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
        let models = resolve(&defaults(), &credentials).unwrap();
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
        let problems = resolve(&defaults(), &credentials).unwrap_err();
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
        let problems = resolve(&defaults(), &credentials).unwrap_err();
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
        let problems = resolve(&configured, &credentials).unwrap_err();
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
