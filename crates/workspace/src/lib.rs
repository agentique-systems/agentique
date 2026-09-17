//! Accepted revisions, reviewed edits and KerML §10 textual project interchange.
use agq_model::*;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    io::{Cursor, Read, Write},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Revision {
    pub id: Id,
    pub parent: Option<Id>,
    pub model: Model,
    pub created: String,
}
impl Revision {
    pub fn import(
        sources: BTreeMap<String, String>,
        identities: &BTreeMap<String, Id>,
    ) -> Result<Self> {
        let model = agq_semantics::compile(sources, identities);
        if !model.accepted() {
            return Err(Error::diagnostics("invalid_model", model.diagnostics));
        }
        Ok(Self {
            id: new_id(),
            parent: None,
            model,
            created: chrono::Utc::now().to_rfc3339(),
        })
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum Edit {
    Rename {
        element_id: Id,
        name: String,
    },
    Move {
        element_id: Id,
        new_owner_id: Id,
    },
    SetValue {
        element_id: Id,
        value: Value,
    },
    SetType {
        element_id: Id,
        type_id: Id,
    },
    AddSource {
        file: String,
        source: String,
    },
    ReplaceSource {
        file: String,
        source: String,
        identity_mapping: BTreeMap<String, Id>,
    },
    Connect {
        owner_id: Id,
        source_id: Id,
        target_id: Id,
    },
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Candidate {
    pub base_revision_id: Id,
    pub edits: Vec<Edit>,
    pub sources: BTreeMap<String, String>,
    pub model: Model,
    pub diff: Vec<FileDiff>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileDiff {
    pub file: String,
    pub before: String,
    pub after: String,
}
pub fn safe_source_name(name: &str) -> bool {
    !name.is_empty()
        && !name.contains(['\\', ':', '\0'])
        && !name
            .split('/')
            .any(|p| p.is_empty() || p == "." || p == "..")
        && (name.ends_with(".sysml") || name.ends_with(".kerml"))
}
fn replace(sources: &mut BTreeMap<String, String>, mut changes: Vec<(Span, String)>) -> Result<()> {
    changes.sort_by(|a, b| (&a.0.file, b.0.start).cmp(&(&b.0.file, a.0.start)));
    changes.dedup();
    let mut previous: BTreeMap<String, usize> = BTreeMap::new();
    for (span, text) in changes {
        let source = sources
            .get_mut(&span.file)
            .ok_or_else(|| Error::new("invalid_edit", "Source not writable"))?;
        if span.end > source.len()
            || !source.is_char_boundary(span.start)
            || !source.is_char_boundary(span.end)
            || previous.get(&span.file).is_some_and(|p| span.end > *p)
        {
            return Err(Error::new(
                "invalid_edit",
                "Overlapping or invalid source spans",
            ));
        }
        previous.insert(span.file.clone(), span.start);
        source.replace_range(span.start..span.end, &text);
    }
    Ok(())
}
fn writable(m: &Model, id: &str) -> Result<Element> {
    let e = m.element(id)?.clone();
    if e.library || e.is_implied {
        return Err(Error::new(
            "permission_denied",
            "Edit the authored declaration; libraries and implied elements are immutable",
        ));
    }
    Ok(e)
}
pub fn propose(base: &Revision, edits: Vec<Edit>) -> Result<Candidate> {
    propose_controlled(base, edits, &WorkControl::default())
}
pub fn propose_controlled(
    base: &Revision,
    edits: Vec<Edit>,
    work: &WorkControl,
) -> Result<Candidate> {
    work.check()?;
    if edits.len() > 100 {
        return Err(Error::new(
            "resource_limit",
            "At most 100 edits per atomic change set",
        ));
    }
    if edits.is_empty() || edits.len() > 100 {
        return Err(Error::new(
            "invalid_edit",
            "A change set requires 1–100 edits",
        ));
    }
    let mut sources = base.model.sources.clone();
    let mut model = base.model.clone();
    let mut identities = agq_semantics::identity_map(&model);
    for edit in &edits {
        work.check()?;
        let mut changes = vec![];
        match edit {
            Edit::Rename { element_id, name } => {
                if name.trim().is_empty() || name.len() > 256 {
                    return Err(Error::new("invalid_edit", "Name must contain 1–256 bytes"));
                }
                let e = writable(&model, element_id)?;
                let span = e
                    .name_span
                    .clone()
                    .ok_or_else(|| Error::new("invalid_edit", "Element is anonymous"))?;
                changes.push((span, agq_syntax::identifier(name)));
                for other in model
                    .elements
                    .values()
                    .filter(|e| !e.library && !e.is_implied)
                {
                    for r in &other.references {
                        for (s, id) in &r.segments {
                            if id == element_id {
                                changes.push((s.clone(), agq_syntax::identifier(name)));
                            }
                        }
                    }
                }
                let new_qn = if let Some(o) = &e.owner {
                    format!("{}::{name}", model.elements[o].qualified_name)
                } else {
                    name.clone()
                };
                for child in model.elements.values().filter(|x| {
                    !x.library
                        && !x.is_implied
                        && (x.id == e.id
                            || x.qualified_name
                                .starts_with(&format!("{}::", e.qualified_name)))
                }) {
                    let key = format!("{}|{}", child.span.file, child.qualified_name);
                    identities.remove(&key);
                    identities.insert(
                        format!(
                            "{}|{}{}",
                            child.span.file,
                            new_qn,
                            &child.qualified_name[e.qualified_name.len()..]
                        ),
                        child.id.clone(),
                    );
                }
            }
            Edit::SetValue { element_id, value } => {
                let e = writable(&model, element_id)?;
                if e.kind != "AttributeUsage" || !value.valid() {
                    return Err(Error::new(
                        "invalid_edit",
                        "SetValue requires an attribute and an exact scalar",
                    ));
                }
                let text = &sources[&e.span.file][e.span.start..e.span.end];
                let (ts, _) = agq_syntax::lex(&e.span.file, text);
                let end = ts.iter().position(|t| t.text == ";").ok_or_else(|| {
                    Error::new(
                        "unsupported_feature",
                        "Attribute body editing is outside this operation",
                    )
                })?;
                let start = ts
                    .iter()
                    .position(|t| t.text == "=")
                    .map(|i| ts[i].span.start)
                    .unwrap_or(ts[end].span.start);
                let literal = match value {
                    Value::Integer(n) => n.clone(),
                    Value::Boolean(v) => v.to_string(),
                    Value::String(v) => serde_json::to_string(v).unwrap(),
                };
                let mut span = e.span.clone();
                span.start += start;
                span.end = e.span.start + ts[end].span.start;
                changes.push((span, format!("= {literal}")));
            }
            Edit::SetType {
                element_id,
                type_id,
            } => {
                let e = writable(&model, element_id)?;
                let t = model.element(type_id)?;
                if !t.definition() {
                    return Err(Error::new("invalid_edit", "Type must be a definition"));
                }
                let r = e.reference("type").ok_or_else(|| {
                    Error::new(
                        "unsupported_feature",
                        "SetType requires explicit existing typing",
                    )
                })?;
                changes.push((r.span.clone(), t.qualified_name.clone()));
            }
            Edit::Move {
                element_id,
                new_owner_id,
            } => {
                let e = writable(&model, element_id)?;
                let owner = writable(&model, new_owner_id)?;
                if owner.id == e.id
                    || owner
                        .qualified_name
                        .starts_with(&format!("{}::", e.qualified_name))
                {
                    return Err(Error::new(
                        "invalid_edit",
                        "Cannot move a namespace into itself",
                    ));
                }
                if e.name.is_none()
                    || !matches!(
                        owner.kind.as_str(),
                        "Package" | "PartDefinition" | "PartUsage"
                    )
                {
                    return Err(Error::new(
                        "unsupported_feature",
                        "Move supports named elements into packages or parts",
                    ));
                }
                let new_qn = format!("{}::{}", owner.qualified_name, e.name.as_ref().unwrap());
                let end = owner
                    .body_end
                    .ok_or_else(|| Error::new("invalid_edit", "Destination needs a body"))?;
                let subtree: BTreeSet<_> = model
                    .elements
                    .values()
                    .filter(|x| {
                        x.id == e.id
                            || x.qualified_name
                                .starts_with(&format!("{}::", e.qualified_name))
                    })
                    .map(|x| x.id.clone())
                    .collect();
                // Qualify references before moving; then reparse to obtain shifted source spans.
                let mut pre = vec![];
                for x in model
                    .elements
                    .values()
                    .filter(|x| !x.library && !x.is_implied)
                {
                    for r in &x.references {
                        if let Some(t) = r.target.as_ref()
                            && (subtree.contains(t) || subtree.contains(&x.id))
                        {
                            let target = &model.elements[t];
                            let qn = if subtree.contains(t) {
                                format!(
                                    "{new_qn}{}",
                                    &target.qualified_name[e.qualified_name.len()..]
                                )
                            } else {
                                target.qualified_name.clone()
                            };
                            pre.push((r.span.clone(), qn));
                        }
                    }
                }
                // Transform the moved slice locally and all remaining references globally in one edit pass.
                let mut moved = sources[&e.span.file][e.span.start..e.span.end].to_string();
                let mut inside: Vec<_> = pre
                    .iter()
                    .filter(|(s, _)| {
                        s.file == e.span.file && s.start >= e.span.start && s.end <= e.span.end
                    })
                    .cloned()
                    .collect();
                inside.sort_by_key(|(s, _)| std::cmp::Reverse(s.start));
                for (s, t) in inside {
                    moved.replace_range(s.start - e.span.start..s.end - e.span.start, &t);
                }
                changes.extend(pre.into_iter().filter(|(s, _)| {
                    !(s.file == e.span.file && s.start >= e.span.start && s.end <= e.span.end)
                }));
                changes.push((e.span.clone(), String::new()));
                let mut insertion = owner.span.clone();
                insertion.start = end;
                insertion.end = end;
                changes.push((insertion, format!("\n{moved}\n")));
                for child in model
                    .elements
                    .values()
                    .filter(|x| !x.is_implied && subtree.contains(&x.id))
                {
                    identities.remove(&format!("{}|{}", child.span.file, child.qualified_name));
                    identities.insert(
                        format!(
                            "{}|{}{}",
                            owner.span.file,
                            new_qn,
                            &child.qualified_name[e.qualified_name.len()..]
                        ),
                        child.id.clone(),
                    );
                }
            }
            Edit::AddSource { file, source } => {
                if !safe_source_name(file) || sources.contains_key(file) {
                    return Err(Error::new(
                        "invalid_edit",
                        "Source path unsafe or already present",
                    ));
                }
                sources.insert(file.clone(), source.clone());
            }
            Edit::ReplaceSource {
                file,
                source,
                identity_mapping,
            } => {
                if !safe_source_name(file) || !sources.contains_key(file) {
                    return Err(Error::new("invalid_edit", "Unknown or unsafe source"));
                }
                sources.insert(file.clone(), source.clone());
                // External text is ambiguous: every old identity must be explicitly reconciled by the caller.
                let old: BTreeSet<_> = model
                    .elements
                    .values()
                    .filter(|e| !e.library && !e.is_implied && e.span.file == *file)
                    .map(|e| e.id.clone())
                    .collect();
                let supplied: BTreeSet<_> = identity_mapping.values().cloned().collect();
                if !old.is_subset(&supplied) {
                    return Err(Error::new(
                        "identity_reconciliation_required",
                        "ReplaceSource must account explicitly for every existing element ID; use typed edits for rename/move",
                    ));
                }
                if identity_mapping
                    .keys()
                    .any(|k| !k.starts_with(&format!("{file}|")))
                {
                    return Err(Error::new(
                        "identity_reconciliation_required",
                        "Identity mapping may only address the replaced source",
                    ));
                }
                identities.retain(|k, _| !k.starts_with(&format!("{file}|")));
                identities.extend(identity_mapping.clone());
            }
            Edit::Connect {
                owner_id,
                source_id,
                target_id,
            } => {
                let o = writable(&model, owner_id)?;
                let a = model.element(source_id)?;
                let b = model.element(target_id)?;
                if a.kind != "PortUsage" || b.kind != "PortUsage" {
                    return Err(Error::new("invalid_edit", "Connection ends must be ports"));
                }
                let end = o
                    .body_end
                    .ok_or_else(|| Error::new("invalid_edit", "Owner needs a body"))?;
                let mut s = o.span.clone();
                s.start = end;
                s.end = end;
                changes.push((
                    s,
                    format!("\nconnect {} to {};\n", a.qualified_name, b.qualified_name),
                ));
            }
        }
        replace(&mut sources, changes)?;
        model = agq_semantics::compile_controlled(sources.clone(), &identities, work);
        work.check()?;
        // Keep invalid candidates inspectable and recoverable; never commit them.
        if !model.accepted() {
            break;
        }
        if let Edit::ReplaceSource {
            identity_mapping, ..
        } = edit
        {
            let actual = agq_semantics::identity_map(&model);
            if identity_mapping
                .iter()
                .any(|(k, v)| actual.get(k) != Some(v))
            {
                return Err(Error::new(
                    "identity_reconciliation_required",
                    "Every reviewed identity must correspond to an element in the new source",
                ));
            }
        }
        identities = agq_semantics::identity_map(&model);
    }
    let diff = sources
        .iter()
        .filter(|(f, s)| base.model.sources.get(*f) != Some(*s))
        .map(|(f, s)| FileDiff {
            file: f.clone(),
            before: base.model.sources.get(f).cloned().unwrap_or_default(),
            after: s.clone(),
        })
        .collect();
    Ok(Candidate {
        base_revision_id: base.id.clone(),
        edits,
        sources,
        model,
        diff,
    })
}
pub fn commit(head: &Revision, candidate: &Candidate) -> Result<Revision> {
    if head.id != candidate.base_revision_id {
        return Err(Error::new(
            "revision_conflict",
            "Proposal base is no longer the accepted revision",
        ));
    }
    if !candidate.model.accepted() {
        return Err(Error::diagnostics(
            "invalid_model",
            candidate.model.diagnostics.clone(),
        ));
    }
    Ok(Revision {
        id: new_id(),
        parent: Some(head.id.clone()),
        model: candidate.model.clone(),
        created: chrono::Utc::now().to_rfc3339(),
    })
}

#[derive(Serialize, Deserialize)]
struct IdentityMetadata {
    format: String,
    source_digest: String,
    identities: BTreeMap<String, Id>,
}
/// Standard source files plus a clearly private identity sidecar. No filesystem access.
pub fn export_text(revision: &Revision) -> Result<BTreeMap<String, String>> {
    if revision.model.sources.keys().any(|f| !safe_source_name(f)) {
        return Err(Error::new("invalid_export", "Unsafe source path"));
    }
    let mut files = revision.model.sources.clone();
    files.insert(
        ".agentique/identities.json".into(),
        serde_json::to_string_pretty(&IdentityMetadata {
            format: "agentique-identity-mapping/0.1".into(),
            source_digest: revision.model.source_digest(),
            identities: agq_semantics::identity_map(&revision.model),
        })
        .map_err(json_error)?,
    );
    Ok(files)
}
pub fn import_text(mut files: BTreeMap<String, String>) -> Result<Revision> {
    let metadata = files
        .remove(".agentique/identities.json")
        .map(|s| serde_json::from_str::<IdentityMetadata>(&s).map_err(json_error))
        .transpose()?;
    if files.is_empty() || files.keys().any(|f| !safe_source_name(f)) {
        return Err(Error::new(
            "invalid_source",
            "Expected only .sysml/.kerml source and optional identity sidecar",
        ));
    }
    let identities = if let Some(m) = metadata {
        if m.format != "agentique-identity-mapping/0.1" || m.source_digest != json_digest(&files) {
            return Err(Error::new(
                "identity_reconciliation_required",
                "Edited source does not match identity sidecar; use an explicit reconciliation change set",
            ));
        }
        if m.identities.values().collect::<BTreeSet<_>>().len() != m.identities.len() {
            return Err(Error::new(
                "invalid_source",
                "Duplicate identity assignment",
            ));
        }
        m.identities
    } else {
        BTreeMap::new()
    };
    Revision::import(files, &identities)
}
pub fn export_kpar(revision: &Revision) -> Result<Vec<u8>> {
    let m = &revision.model;
    let mut archive = zip::ZipWriter::new(Cursor::new(vec![]));
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let mut index = BTreeMap::new();
    let mut checksums = BTreeMap::new();
    let mut languages = BTreeSet::new();
    for (file, source) in &m.sources {
        if !safe_source_name(file) {
            return Err(Error::new("invalid_export", "Unsafe source path"));
        }
        languages.insert(file.rsplit('.').next().unwrap());
        archive.start_file(file, options).map_err(zip_error)?;
        archive.write_all(source.as_bytes()).map_err(io_error)?;
        checksums.insert(
            file,
            serde_json::json!([{"algorithm":"SHA256","value":digest(source)}]),
        );
    }
    if languages.len() != 1 {
        return Err(Error::new(
            "unsupported_feature",
            "KerML §10.3 requires one language per project archive",
        ));
    }
    for e in m
        .elements
        .values()
        .filter(|e| !e.library && e.owner.is_none())
    {
        if let Some(n) = &e.name {
            index.insert(n.clone(), e.span.file.clone());
        }
        if let Some(n) = &e.short_name {
            index.insert(n.clone(), e.span.file.clone());
        }
    }
    let lock: serde_json::Value = serde_json::from_str(agq_semantics::LIBRARY_LOCK).unwrap();
    let usage:Vec<_>=lock["artifacts"].as_array().unwrap().iter().filter(|a|a["path"].as_str().is_some_and(|p|p.ends_with(".kpar"))).map(|a|serde_json::json!({"resource":a["source"],"versionConstraint":if a["id"]=="sysml-systems-library"{"2.0.0"}else{"1.0.0"}})).collect();
    let project = serde_json::json!({"name":"Agentique","version":"0.1.0","usage":usage});
    let meta = serde_json::json!({"index":index,"created":chrono::Utc::now().to_rfc3339(),"metamodel":if languages.contains("sysml"){"https://www.omg.org/spec/SysML/20250201"}else{"https://www.omg.org/spec/KerML/20250201"},"checksum":checksums});
    let identities = IdentityMetadata {
        format: "agentique-identity-mapping/0.1".into(),
        source_digest: m.source_digest(),
        identities: agq_semantics::identity_map(m),
    };
    for (name, bytes) in [
        (
            ".project.json",
            serde_json::to_vec_pretty(&project).unwrap(),
        ),
        (".meta.json", serde_json::to_vec_pretty(&meta).unwrap()),
        (
            ".agentique/identities.json",
            serde_json::to_vec_pretty(&identities).unwrap(),
        ),
    ] {
        archive.start_file(name, options).map_err(zip_error)?;
        archive.write_all(&bytes).map_err(io_error)?;
    }
    Ok(archive.finish().map_err(zip_error)?.into_inner())
}
pub fn import_kpar(bytes: &[u8]) -> Result<Revision> {
    if bytes.len() > 32 * 1024 * 1024 {
        return Err(Error::new("resource_limit", "Archive exceeds 32 MiB"));
    }
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes)).map_err(zip_error)?;
    let mut files = BTreeMap::new();
    let mut total = 0;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(zip_error)?;
        let name = entry.name().to_string();
        if entry.is_dir() {
            continue;
        }
        if name.contains(['\\', ':', '\0']) || name.split('/').any(|p| p == ".." || p.is_empty()) {
            return Err(Error::new("invalid_archive", "Unsafe archive path"));
        }
        total += entry.size();
        if total > 64 * 1024 * 1024 || files.len() > 2000 {
            return Err(Error::new(
                "resource_limit",
                "Expanded archive exceeds limits",
            ));
        }
        let mut data = String::new();
        entry.read_to_string(&mut data).map_err(io_error)?;
        if files.insert(name, data).is_some() {
            return Err(Error::new("invalid_archive", "Duplicate entry"));
        }
    }
    let project: serde_json::Value =
        serde_json::from_str(files.get(".project.json").ok_or_else(|| {
            Error::new(
                "invalid_archive",
                "Missing top-level .project.json (KerML 10.3)",
            )
        })?)
        .map_err(json_error)?;
    let meta: serde_json::Value = serde_json::from_str(
        files
            .get(".meta.json")
            .ok_or_else(|| Error::new("invalid_archive", "Missing top-level .meta.json"))?,
    )
    .map_err(json_error)?;
    if !project["name"].is_string()
        || !project["version"].is_string()
        || !meta["index"].is_object()
        || !meta["created"].is_string()
    {
        return Err(Error::new(
            "invalid_archive",
            "Required standard project metadata missing",
        ));
    }
    let metamodel = meta["metamodel"]
        .as_str()
        .unwrap_or("https://www.omg.org/spec/KerML/20250201");
    let ext = match metamodel {
        "https://www.omg.org/spec/SysML/20250201" => "sysml",
        "https://www.omg.org/spec/KerML/20250201" => "kerml",
        _ => {
            return Err(Error::new(
                "unsupported_feature",
                "Unsupported metamodel revision",
            ));
        }
    };
    if let Some(usage) = project["usage"].as_array() {
        let lock: serde_json::Value = serde_json::from_str(agq_semantics::LIBRARY_LOCK).unwrap();
        for u in usage {
            let artifact = lock["artifacts"]
                .as_array()
                .unwrap()
                .iter()
                .find(|a| a["source"] == u["resource"])
                .ok_or_else(|| {
                    Error::new(
                        "unresolved_reference",
                        "Project dependency is not in the pinned library closure",
                    )
                })?;
            let version = if artifact["id"] == "sysml-systems-library" {
                "2.0.0"
            } else {
                "1.0.0"
            };
            if u["versionConstraint"]
                .as_str()
                .is_some_and(|s| s != version)
            {
                return Err(Error::new(
                    "unsupported_feature",
                    "Dependency version constraint is not the pinned exact version",
                ));
            }
        }
    }
    let sources: BTreeMap<_, _> = files
        .iter()
        .filter(|(f, _)| safe_source_name(f))
        .map(|(f, s)| (f.clone(), s.clone()))
        .collect();
    for name in files.keys() {
        if !safe_source_name(name)
            && ![".project.json", ".meta.json", ".agentique/identities.json"]
                .contains(&name.as_str())
        {
            return Err(Error::new(
                "unsupported_feature",
                format!(
                    "Archive entry {name} cannot be retained by textual interchange; import refused to prevent loss"
                ),
            ));
        }
    }
    if sources.is_empty() || sources.keys().any(|f| !f.ends_with(ext)) {
        return Err(Error::new(
            "invalid_archive",
            "Missing source or mixed project languages",
        ));
    }
    for value in meta["index"].as_object().unwrap().values() {
        if !value.as_str().is_some_and(|s| sources.contains_key(s)) {
            return Err(Error::new(
                "invalid_archive",
                "Index references missing source",
            ));
        }
    }
    for (file, source) in &sources {
        if let Some(cs) = meta["checksum"][file].as_array() {
            for c in cs {
                if c["algorithm"] != "SHA256" {
                    return Err(Error::new(
                        "unsupported_feature",
                        "Only SHA256 project checksums supported",
                    ));
                }
                if c["value"] != digest(source) {
                    return Err(Error::new(
                        "corrupt_data",
                        "Project source checksum mismatch",
                    ));
                }
            }
        }
    }
    let identities = if let Some(s) = files.get(".agentique/identities.json") {
        let metadata: IdentityMetadata = serde_json::from_str(s).map_err(json_error)?;
        if metadata.format != "agentique-identity-mapping/0.1"
            || metadata.source_digest != json_digest(&sources)
        {
            return Err(Error::new(
                "identity_reconciliation_required",
                "Identity sidecar does not match source bytes",
            ));
        }
        let values: BTreeSet<_> = metadata.identities.values().collect();
        if values.len() != metadata.identities.len() {
            return Err(Error::new(
                "invalid_archive",
                "Duplicate identity assignment",
            ));
        }
        metadata.identities
    } else {
        BTreeMap::new()
    };
    Revision::import(sources, &identities)
}
fn zip_error(e: zip::result::ZipError) -> Error {
    Error::new("invalid_archive", e.to_string())
}
fn io_error(e: std::io::Error) -> Error {
    Error::new("io_error", e.to_string())
}
fn json_error(e: serde_json::Error) -> Error {
    Error::new("invalid_archive", e.to_string())
}
