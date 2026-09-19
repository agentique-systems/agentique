//! Compiled descriptor evidence for the independent direct-XMI verifier.
use agq_kerml::{BaselineProfile, descriptors_for_profile};
use serde_json::{Value, json};

fn descriptors(profile: BaselineProfile) -> Value {
    let set = descriptors_for_profile(profile).unwrap();
    json!({
        "profile": profile.id(),
        "sources":set.sources.iter().map(|(id,s)| (format!("{id:?}"),json!({
            "external_id":s.external_id,"artifact_uri":s.artifact_uri,"sha256":s.sha256,
            "byte_range":s.byte_range,"specification":s.specification,"version":s.version
        }))).collect::<serde_json::Map<_,_>>(),
        "models":format!("{:?}",set.models),"classes":format!("{:?}",set.classes),
        "enumerations":format!("{:?}",set.enumerations),"primitives":format!("{:?}",set.primitives),
        "reviews":format!("{:?}",set.reviews),
        "properties":set.properties.iter().map(|p|(p.id.to_string(),json!(format!("{p:?}")))).collect::<serde_json::Map<_,_>>(),
        "associations":set.associations.iter().map(|a|(a.id.to_string(),json!(format!("{a:?}")))).collect::<serde_json::Map<_,_>>()
    })
}
fn main() {
    println!(
        "{}",
        json!({"published":descriptors(BaselineProfile::PublishedKerMl10),
        "operational":descriptors(BaselineProfile::OPERATIONAL)})
    );
}
