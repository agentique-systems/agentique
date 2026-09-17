//! Bounded read mapping of Systems Modeling API 1.0. Simulation is never routed here.
use super::*;
#[derive(Default, Deserialize)]
pub struct Paging {
    #[serde(rename = "page[after]")]
    after: Option<String>,
    #[serde(rename = "page[before]")]
    before: Option<String>,
    #[serde(rename = "page[size]")]
    size: Option<usize>,
    #[serde(rename = "excludeUsed", default)]
    exclude_used: bool,
}
fn page(mut values: Vec<Value>, q: Paging, url: &str) -> std::result::Result<Response, ApiError> {
    values.sort_by(|a, b| a["@id"].as_str().cmp(&b["@id"].as_str()));
    let size = q.size.unwrap_or(100);
    if size == 0 || size > 1000 || (q.after.is_some() && q.before.is_some()) {
        return Err(Error::new(
            "invalid_page",
            "Use one cursor and page[size] between 1 and 1000",
        )
        .into());
    }
    let position = |id: &str| {
        values.iter().position(|v| v["@id"] == id).ok_or_else(|| {
            ApiError(Error::new(
                "invalid_page",
                "Cursor does not belong to this collection",
            ))
        })
    };
    let end = q
        .before
        .as_deref()
        .map(position)
        .transpose()?
        .unwrap_or(values.len());
    let start = if let Some(id) = q.after.as_deref() {
        position(id)? + 1
    } else if q.before.is_some() {
        end.saturating_sub(size)
    } else {
        0
    };
    let end = end.min(start.saturating_add(size));
    let slice = &values[start.min(end)..end];
    let mut links = vec![];
    let suffix = format!("page[size]={size}&excludeUsed={}", q.exclude_used);
    if end < values.len()
        && let Some(last) = slice.last()
    {
        links.push(format!(
            "<{url}?page[after]={}&{suffix}>; rel=\"next\"",
            last["@id"].as_str().unwrap()
        ));
    }
    if start > 0
        && let Some(first) = slice.first()
    {
        links.push(format!(
            "<{url}?page[before]={}&{suffix}>; rel=\"prev\"",
            first["@id"].as_str().unwrap()
        ));
    }
    let mut response = Json(json!(slice)).into_response();
    if !links.is_empty() {
        response
            .headers_mut()
            .insert(header::LINK, links.join(", ").parse().unwrap());
    }
    Ok(response)
}
fn project(a: &Application) -> Result<Value> {
    let first = a
        .store
        .list("revision")?
        .into_iter()
        .min_by_key(|r| r["created"].as_str().unwrap_or("").to_string())
        .unwrap();
    Ok(
        json!({"@id":a.project_id,"@type":"Project","alias":[],"name":"Agentique","description":"Local Agentique project","created":first["created"],"defaultBranch":{"@id":agq_model::derived_id(&a.project_id,"main")}}),
    )
}
fn branch(a: &Application) -> Result<Value> {
    Ok(
        json!({"@id":agq_model::derived_id(&a.project_id,"main"),"@type":"Branch","alias":[],"name":"main","description":"Accepted model revisions","created":project(a)?["created"],"deleted":null,"head":{"@id":a.head.id},"referencedCommit":{"@id":a.head.id},"owningProject":{"@id":a.project_id}}),
    )
}
fn commit(r: &agq_workspace::Revision, p: &str) -> Value {
    json!({"@id":r.id,"@type":"Commit","alias":[],"name":null,"description":null,"owningProject":{"@id":p},"created":r.created,"previousCommit":r.parent.as_ref().map(|id|json!({"@id":id})).into_iter().collect::<Vec<_>>()})
}
fn element_value(e: &agq_model::Element) -> Value {
    json!({"@id":e.id,"@type":e.metaclass(),"elementId":e.id,"declaredName":e.name,"declaredShortName":e.short_name,"qualifiedName":e.qualified_name,"owner":e.owner.as_ref().map(|id|json!({"@id":id})),"isImpliedIncluded":false})
}
async fn projects(
    State(s): State<Server>,
    Query(q): Query<Paging>,
) -> std::result::Result<Response, ApiError> {
    let data = work(s, |a| Ok(vec![project(a)?])).await?;
    page(data, q, "/api/model/projects")
}
async fn get_project(State(s): State<Server>, Path(p): Path<String>) -> ApiResult<Value> {
    Ok(Json(
        work(s, move |a| {
            require_project(a, &p)?;
            project(a)
        })
        .await?,
    ))
}
async fn branches(
    State(s): State<Server>,
    Path(p): Path<String>,
    Query(q): Query<Paging>,
) -> std::result::Result<Response, ApiError> {
    let url = format!("/api/model/projects/{p}/branches");
    let data = work(s, move |a| {
        require_project(a, &p)?;
        Ok(vec![branch(a)?])
    })
    .await?;
    page(data, q, &url)
}
async fn get_branch(
    State(s): State<Server>,
    Path((p, b)): Path<(String, String)>,
) -> ApiResult<Value> {
    Ok(Json(
        work(s, move |a| {
            require_project(a, &p)?;
            if b != agq_model::derived_id(&p, "main") {
                return Err(Error::new("not_found", "Unknown branch"));
            }
            branch(a)
        })
        .await?,
    ))
}
async fn commits(
    State(s): State<Server>,
    Path(p): Path<String>,
    Query(q): Query<Paging>,
) -> std::result::Result<Response, ApiError> {
    let url = format!("/api/model/projects/{p}/commits");
    let data = work(s, move |a| {
        require_project(a, &p)?;
        a.store
            .list("revision")?
            .into_iter()
            .map(|r| {
                serde_json::from_value(r)
                    .map(|r| commit(&r, &p))
                    .map_err(|e| Error::new("corrupt_data", e.to_string()))
            })
            .collect()
    })
    .await?;
    page(data, q, &url)
}
async fn get_commit(
    State(s): State<Server>,
    Path((p, c)): Path<(String, String)>,
) -> ApiResult<Value> {
    Ok(Json(
        work(s, move |a| {
            require_project(a, &p)?;
            Ok(commit(&a.revision(&c)?, &p))
        })
        .await?,
    ))
}
async fn elements(
    State(s): State<Server>,
    Path((p, c)): Path<(String, String)>,
    Query(q): Query<Paging>,
) -> std::result::Result<Response, ApiError> {
    let url = format!("/api/model/projects/{p}/commits/{c}/elements");
    let exclude = q.exclude_used;
    let data = work(s, move |a| {
        require_project(a, &p)?;
        Ok(a.revision(&c)?
            .model
            .elements
            .values()
            .filter(|e| !e.is_implied && (!exclude || !e.library))
            .map(element_value)
            .collect())
    })
    .await?;
    page(data, q, &url)
}
async fn element(
    State(s): State<Server>,
    Path((p, c, e)): Path<(String, String, String)>,
    Query(q): Query<Paging>,
) -> ApiResult<Value> {
    Ok(Json(
        work(s, move |a| {
            require_project(a, &p)?;
            let r = a.revision(&c)?;
            let e = r.model.element(&e)?;
            if e.is_implied || (q.exclude_used && e.library) {
                return Err(Error::new(
                    "not_found",
                    "Element outside requested projection",
                ));
            }
            Ok(element_value(e))
        })
        .await?,
    ))
}
async fn roots(
    State(s): State<Server>,
    Path((p, c)): Path<(String, String)>,
    Query(q): Query<Paging>,
) -> std::result::Result<Response, ApiError> {
    let url = format!("/api/model/projects/{p}/commits/{c}/roots");
    let exclude = q.exclude_used;
    let data = work(s, move |a| {
        require_project(a, &p)?;
        Ok(a.revision(&c)?
            .model
            .elements
            .values()
            .filter(|e| !e.is_implied && (!exclude || !e.library) && e.owner.is_none())
            .map(element_value)
            .collect())
    })
    .await?;
    page(data, q, &url)
}
pub fn routes() -> Router<Server> {
    Router::new()
        .route("/projects", get(projects))
        .route("/projects/{p}", get(get_project))
        .route("/projects/{p}/branches", get(branches))
        .route("/projects/{p}/branches/{b}", get(get_branch))
        .route("/projects/{p}/commits", get(commits))
        .route("/projects/{p}/commits/{c}", get(get_commit))
        .route("/projects/{p}/commits/{c}/elements", get(elements))
        .route("/projects/{p}/commits/{c}/elements/{e}", get(element))
        .route("/projects/{p}/commits/{c}/roots", get(roots))
}
