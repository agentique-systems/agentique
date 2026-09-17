//! Bounded, cancellable application command work. A queued response is never a commit receipt.
use super::*;
use agq_model::{WorkControl, json_digest};

#[derive(Clone)]
pub struct Job {
    digest: String,
    control: WorkControl,
    result: Option<Result<Value>>,
}
pub type Jobs = Arc<Mutex<BTreeMap<String, Job>>>;
pub fn routes() -> Router<Server> {
    Router::new()
        .route("/jobs", post(submit))
        .route("/jobs/{id}", get(read))
        .route("/jobs/{id}/cancel", post(cancel))
}
async fn submit(State(s): State<Server>, Json(command): Json<Command>) -> ApiResult<Value> {
    let id = command.command_id.clone();
    if id.is_empty() || id.len() > 128 {
        return Err(Error::new("invalid_command", "Invalid command ID").into());
    }
    let digest = json_digest(&command);
    let control = WorkControl::default();
    {
        let mut jobs = s.jobs.lock().unwrap();
        if let Some(job) = jobs.get(&id) {
            if job.digest != digest {
                return Err(Error::new(
                    "idempotency_conflict",
                    "Job ID reused with altered command",
                )
                .into());
            }
            return Ok(Json(json!({"job_id":id,"acknowledged_commit":false})));
        }
        if jobs.values().filter(|j| j.result.is_none()).count() >= 8 {
            return Err(
                Error::new("resource_limit", "Eight application jobs already pending").into(),
            );
        }
        while jobs.len() >= 64 {
            let key = jobs
                .iter()
                .find(|(_, j)| j.result.is_some())
                .map(|(id, _)| id.clone())
                .unwrap();
            jobs.remove(&key);
        }
        jobs.insert(
            id.clone(),
            Job {
                digest,
                control: control.clone(),
                result: None,
            },
        );
    }
    let job_id = id.clone();
    tokio::spawn(async move {
        let result = work(s.clone(), move |a| {
            a.command_controlled(Actor::Operator, command, &control)
        })
        .await
        .map_err(|e| e.0);
        if let Some(job) = s.jobs.lock().unwrap().get_mut(&job_id) {
            job.result = Some(result);
        }
    });
    Ok(Json(json!({"job_id":id,"acknowledged_commit":false})))
}
async fn read(State(s): State<Server>, Path(id): Path<String>) -> ApiResult<Value> {
    let jobs = s.jobs.lock().unwrap();
    let job = jobs.get(&id).ok_or_else(|| {
        Error::new(
            "not_found",
            "Job unavailable; retry the original command ID to recover a durable receipt",
        )
    })?;
    Ok(Json(match &job.result {
        Some(Ok(result)) => {
            json!({"job_id":id,"status":"committed","result":result,"progress":job.control.progress()})
        }
        Some(Err(error)) => {
            json!({"job_id":id,"status":if error.code=="cancelled" {"cancelled"} else {"failed"},"error":error,"progress":job.control.progress()})
        }
        None => json!({"job_id":id,"status":"working","progress":job.control.progress()}),
    }))
}
async fn cancel(State(s): State<Server>, Path(id): Path<String>) -> ApiResult<Value> {
    let jobs = s.jobs.lock().unwrap();
    let job = jobs
        .get(&id)
        .ok_or_else(|| Error::new("not_found", "Unknown job"))?;
    let accepted = job.result.is_none() && job.control.cancel();
    Ok(Json(
        json!({"job_id":id,"cancellation_requested":accepted,"message":if accepted {"Cancellation requested; inspect final job outcome"} else {"Commit already started or job finished; inspect final outcome"}}),
    ))
}
