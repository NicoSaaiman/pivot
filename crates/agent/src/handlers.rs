use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};

use crate::{
    errors::Result,
    info, 
    metrics,
    state::AppState,
};

use shared::models::{
    Workload, 
    WorkloadId,
};

pub async fn execute_workload(
    State(state): State<AppState>,
    Json(workload): Json<Workload>,
) -> Result<impl IntoResponse> {
    state
        .runtime
        .execute_workload(workload)
        .await?;

    Ok(Json("Workload started"))
}

pub async fn get_workload(
    State(state): State<AppState>,    
    Path(workload_id): Path<WorkloadId>,
) -> Result<impl IntoResponse> {
    let workload_status = state
        .runtime.
        get_workload(workload_id)
        .await?;

    Ok(Json(workload_status))
}

pub async fn get_workloads(
        State(state): State<AppState>,    
) -> Result<impl IntoResponse> {
    let workloads = state
        .runtime
        .get_workloads()
        .await?;
    
    Ok(Json(workloads))
}

pub async fn delete_workload(
    State(state): State<AppState>,
    Path(workload_id): Path<WorkloadId>,
) -> Result<impl IntoResponse> {
    let result = state
        .runtime
        .delete_workload(workload_id)
        .await?;

    Ok(Json(result))
}

pub async fn get_node_info() -> Result<impl IntoResponse> {
    let info = info::get_node_info()?;

    Ok(Json(info))
}

pub async fn get_node_metrics() -> Result<impl IntoResponse> {
    let metrics = metrics::get_node_metrics()?;

    Ok(Json(metrics))
}
