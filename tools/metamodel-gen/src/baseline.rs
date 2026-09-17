//! Reviewed baselines, independent of filenames and XMI serialization profiles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DescriptorTarget {
    KerMlRootCore,
    SysMlStructural,
    None,
}
#[derive(Clone, Copy, Debug)]
pub struct Baseline {
    pub id: &'static str,
    pub specification: &'static str,
    pub version: &'static str,
    pub metamodel_uri: &'static str,
    pub serialized_root_uri: &'static str,
    pub input_lock: &'static str,
    pub primary_xmi: &'static str,
    pub primary_sha256: &'static str,
    pub cross_check_json: Option<&'static str>,
    pub dependencies: &'static [&'static str],
    pub output: Option<&'static str>,
    pub descriptor_target: DescriptorTarget,
    pub descriptor_outputs: &'static [&'static str],
}
pub const PRIMITIVES: Baseline = Baseline {
    id: "uml-primitives-2.5.1",
    specification: "UML",
    version: "2.5.1",
    metamodel_uri: "http://www.omg.org/spec/PrimitiveTypes/20161101",
    serialized_root_uri: "http://www.omg.org/spec/PrimitiveTypes/20161101",
    input_lock: "standards/normative/kerml-1.0/lock.json",
    primary_xmi: "PrimitiveTypes.xmi",
    primary_sha256: "62d12217fcd26037fc917709e2a896600af574efd6412c110e2a711395a69849",
    cross_check_json: None,
    dependencies: &[],
    output: None,
    descriptor_target: DescriptorTarget::None,
    descriptor_outputs: &[],
};
pub const KERML: Baseline = Baseline {
    id: "kerml-1.0",
    specification: "KerML",
    version: "1.0",
    metamodel_uri: "https://www.omg.org/spec/KerML/20250201",
    serialized_root_uri: "https://www.omg.org/spec/KerML/20250201",
    input_lock: "standards/normative/kerml-1.0/lock.json",
    primary_xmi: "KerML.xmi",
    primary_sha256: "45b18775afe2b2fcdc70e24f37c6d2f344defcc3f38a02075a193354e2d7b466",
    cross_check_json: Some("KerML.json"),
    dependencies: &[PRIMITIVES.id],
    output: Some("standards/generated/kerml-1.0/metamodel.json"),
    descriptor_target: DescriptorTarget::KerMlRootCore,
    descriptor_outputs: &[
        "crates/kerml/src/generated/root_core.rs",
        "standards/generated/kerml-1.0/root-core.golden.json",
        "crates/kerml/src/generated/typed_views.rs",
    ],
};
pub const SYSML: Baseline = Baseline {
    id: "sysml-2.0",
    specification: "SysML",
    version: "2.0",
    metamodel_uri: "https://www.omg.org/spec/SysML/20250201",
    // Exact published spelling, not an alias for resolving external references.
    serialized_root_uri: "https://ww.omg.org/spec/SysML/20250201",
    input_lock: "standards/normative/sysml-2.0/lock.json",
    primary_xmi: "SysML.xmi",
    primary_sha256: "caa65d54f56798bf7582d173f7567e1eea37a49c45984f8bd7df145011cf8c6f",
    cross_check_json: Some("SysML.json"),
    dependencies: &[PRIMITIVES.id, KERML.id],
    output: Some("standards/generated/sysml-2.0/metamodel.json"),
    descriptor_target: DescriptorTarget::SysMlStructural,
    descriptor_outputs: &[
        "crates/sysml/src/generated/structural.rs",
        "standards/generated/sysml-2.0/structural.golden.json",
        "crates/sysml/src/generated/typed_views.rs",
    ],
};
pub fn find(id: &str) -> crate::Result<Baseline> {
    [PRIMITIVES, KERML, SYSML]
        .into_iter()
        .find(|b| b.id == id)
        .ok_or_else(|| format!("unsupported normative baseline {id}"))
}
