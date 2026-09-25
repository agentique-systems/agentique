//! Same immutable canonical query context, old two-pass delivery versus observer.
//! Accepted runtime inputs are mandatory; no fixture context grants acceptance.
use super::*;
use crate::{ProjectChange, SourceCompilation, SourceDiagnostic, SourceInputs, SourceLanguage};

#[path = "../../../../verification/fixtures/modeling-workspace-phase1/working_state_inputs.rs"]
mod inputs;

fn capability(
    compilation: &SourceCompilation,
    subject: ElementId,
    answer: SysmlQueryResult<Vec<ElementId>>,
) -> Option<SourceDiagnostic> {
    (answer.completeness() != Completeness::Complete).then(|| SourceDiagnostic::Capability {
        subject,
        origin: compilation
            .source_map()
            .get(&FactKey::Element(subject))
            .cloned(),
        answer: Box::new(answer),
    })
}

fn append_findings(
    compilation: &SourceCompilation,
    report: &SystemsPublicationAudit,
    diagnostics: &mut Vec<SourceDiagnostic>,
) {
    for finding in &report.findings {
        let origin = match finding {
            SystemsPublicationFinding::Capability { diagnostic, .. } => compilation
                .source_map()
                .get(&FactKey::Element(diagnostic.subject))
                .cloned(),
            _ => None,
        };
        diagnostics.push(SourceDiagnostic::EffectiveAudit {
            origin,
            finding: Box::new(finding.clone()),
        });
    }
}

fn assert_population_parity(compilation: &SourceCompilation) {
    let bound = compilation.sysml_queries().unwrap();
    let standard = compilation.inputs().accepted_sysml().overlay().model();
    let mut subjects: Vec<_> = bound
        .model()
        .elements()
        .filter(|record| standard.element(record.id()).is_none())
        .map(|record| record.id())
        .collect();
    subjects.sort_unstable();
    assert_eq!(compilation.effective_audit().unwrap().subjects(), subjects);
    let mut old_report = SystemsPublicationAudit::default();
    let mut old_diagnostics = Vec::new();
    let mut old_answers = Vec::new();
    // Exactly the previous implementation: all audit operations in this batch,
    // then a second effective_usages call for each Definition or Usage.
    for batch in subjects.chunks(32) {
        let q = bound.fork();
        audit_sysml_population(&q, batch, &mut old_report);
        for &subject in batch {
            let class = q.model().element(subject).unwrap().metaclass();
            if [sc::DEFINITION, sc::USAGE].into_iter().any(|base| {
                q.model()
                    .registry()
                    .is_subtype(class, base)
                    .unwrap_or(false)
            }) {
                let answer = q.effective_usages(subject);
                old_answers.push((subject, format!("{answer:?}")));
                old_diagnostics.extend(capability(compilation, subject, answer));
            }
        }
    }
    append_findings(compilation, &old_report, &mut old_diagnostics);
    let mut observed_report = SystemsPublicationAudit::default();
    let mut observed_diagnostics = Vec::new();
    let mut observed_answers = Vec::new();
    for batch in subjects.chunks(32) {
        let q = bound.fork();
        audit_authored_effective_population(&q, batch, &mut observed_report, |subject, answer| {
            observed_answers.push((subject, format!("{answer:?}")));
            if answer.completeness() != Completeness::Complete {
                observed_diagnostics.extend(capability(compilation, subject, answer.clone()));
            }
        });
    }
    append_findings(compilation, &observed_report, &mut observed_diagnostics);
    // Derived Debug includes every public context/evidence field and the private
    // additional_completeness field. There is no revision normalization here:
    // these answers are over the exact same immutable context and canonical IDs.
    assert_eq!(
        old_answers, observed_answers,
        "exact answer and callback subject order"
    );
    assert_eq!(format!("{old_report:?}"), format!("{observed_report:?}"));
    assert_eq!(
        format!("{old_diagnostics:?}"),
        format!("{observed_diagnostics:?}")
    );
    assert_eq!(
        format!("{old_report:?}"),
        format!("{:?}", compilation.effective_audit().unwrap().report())
    );
    let stored: Vec<_> = compilation
        .diagnostics()
        .iter()
        .filter(|diagnostic| {
            matches!(
                diagnostic,
                SourceDiagnostic::Capability { .. } | SourceDiagnostic::EffectiveAudit { .. }
            )
        })
        .collect();
    assert_eq!(
        format!("{old_diagnostics:?}"),
        format!("{stored:?}"),
        "actual source diagnostics preserve capability-then-audit ordering"
    );
}

/// Exercise delivery of actual Complete, Incomplete and Invalid public answers.
/// The invalid-kind probe tests this private delivery boundary deliberately;
/// it does not broaden the production dispatcher's Definition/Usage guard.
fn assert_answer_delivery(
    compilation: &SourceCompilation,
    subject: ElementId,
    expected: Completeness,
) {
    let bound = compilation.sysml_queries().unwrap();
    let q = bound.fork();
    let families = [
        SystemsPublicationFamily::DefinitionUsage,
        SystemsPublicationFamily::AttributeItemPart,
    ];
    let mut old_report = SystemsPublicationAudit::default();
    audit_typed_answer(
        &mut old_report,
        &families,
        subject,
        "effective usages",
        q.effective_usages(subject),
    );
    let old_answer = q.effective_usages(subject);
    assert_eq!(
        old_answer.completeness(),
        expected,
        "subject {subject:?}, present={}, full answer={old_answer:?}",
        q.model().element(subject).is_some(),
    );
    if q.model().element(subject).is_none() {
        // The local SysML check records missing evidence, but the composed
        // KerML typed-subject query is Invalid; the public result retains the
        // strongest completeness from every contributing answer.
        assert_eq!(old_answer.kerml.completeness, Completeness::Invalid);
        assert!(old_answer.diagnostics.iter().any(|diagnostic| {
            diagnostic.code == "SQ_MISSING_ELEMENT" && diagnostic.subject == subject
        }));
    }
    let old_exact = format!("{old_answer:?}");
    let old_diagnostic = capability(compilation, subject, old_answer);
    let q = bound.fork();
    let mut observed_report = SystemsPublicationAudit::default();
    let mut observed_exact = None;
    let mut observed_diagnostic = None;
    audit_effective_usages(
        &q,
        &mut observed_report,
        &families,
        subject,
        &mut |id, answer| {
            assert_eq!(id, subject);
            assert_eq!(answer.completeness(), expected);
            assert!(
                observed_exact.is_none(),
                "one delivery per evaluated answer"
            );
            observed_exact = Some(format!("{answer:?}"));
            if answer.completeness() != Completeness::Complete {
                observed_diagnostic = capability(compilation, subject, answer.clone());
            }
        },
    );
    assert_eq!(Some(old_exact), observed_exact);
    assert_eq!(
        format!("{old_diagnostic:?}"),
        format!("{observed_diagnostic:?}")
    );
    assert_eq!(format!("{old_report:?}"), format!("{observed_report:?}"));
    if expected == Completeness::Complete {
        assert!(observed_diagnostic.is_none());
        assert!(observed_report.findings.is_empty());
        assert!(
            families
                .iter()
                .all(|family| observed_report.checked.get(family) == Some(&1))
        );
    } else {
        assert!(observed_diagnostic.is_some());
        assert!(!observed_report.findings.is_empty());
    }
}

#[test]
#[ignore = "requires exact accepted runtime caches; never acquires or rebuilds standards"]
fn authored_audit_observer_matches_two_pass_queries_and_diagnostics() {
    use crate::sysml::CanonicalSysmlSystemsLibrary;
    use std::{fs::File, path::Path};
    // Fail before expensive restoration when either supplied cache is absent.
    let kerml_file = File::open(
        std::env::var_os("AGENTIQUE_KERML_CACHE").expect("accepted KerML cache required"),
    )
    .unwrap();
    let systems_file = File::open(
        std::env::var_os("AGENTIQUE_SYSTEMS_CACHE").expect("accepted Systems cache required"),
    )
    .unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let sources = VerifiedLibrarySet::load_from_directory(&root).unwrap();
    let kerml =
        Arc::new(CanonicalKermlStandardLibraries::restore_cache(kerml_file, &sources).unwrap());
    let accepted = Arc::new(
        CanonicalSysmlSystemsLibrary::restore_cache(systems_file, &sources, kerml).unwrap(),
    );
    let base = SourceInputs::with_accepted_sysml(accepted).unwrap();
    for (source, expected) in [
        (inputs::ORDINARY_DOCUMENT, Completeness::Complete),
        (inputs::VARIATION_DOCUMENT, Completeness::Incomplete),
    ] {
        let current = Arc::new(
            base.apply([ProjectChange::Add {
                path: "Choices.sysml".into(),
                language: SourceLanguage::SysMl,
                source: source.into(),
            }])
            .unwrap(),
        );
        let compilation = current.compile(None).unwrap();
        assert_population_parity(&compilation);
        let q = compilation.kerml_queries().unwrap();
        let answer = q.lookup_path(
            current.root(),
            &agq_kerml_semantics::QualifiedName {
                absolute: false,
                segments: vec!["Choices".into(), "Configurable".into()],
            },
        );
        let ids: std::collections::BTreeSet<_> =
            answer.value.iter().map(|target| target.element).collect();
        assert_eq!(ids.len(), 1);
        assert_answer_delivery(&compilation, *ids.first().unwrap(), expected);
        // Root is an existing canonical Package, not a Definition/Usage. A
        // missing identity also has Invalid composed completeness from KerML's
        // typed-subject check, alongside the SysML missing-element diagnostic.
        assert_answer_delivery(&compilation, current.root(), Completeness::Invalid);
        assert_answer_delivery(&compilation, ElementId::new(), Completeness::Invalid);
    }
}
