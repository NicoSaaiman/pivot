use std::sync::{Arc, RwLock};

use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};

use crate::agent_client::AgentClient;

use crate::{
    errors::{ControllerError, Result},
    scheduler::ScheduleManager,
    state::ControllerState
};

use shared::models::{Node, Workload, WorkloadId};

pub async fn execute_workload(
    State(state): State<Arc<RwLock<ControllerState>>>,
    Json(workload): Json<Workload>,
) -> Result<impl IntoResponse> {
    let workload_id = workload.get_workload_id();
    let schedule_manager = ScheduleManager::build(state, workload)?;

    schedule_manager
        .schedule_workload()
        .await?;

    let node = schedule_manager.get_node();
    tracing::info!("Workload scheduled: {workload_id} -> {}", node.id);

    Ok(Json("Workload scheduled"))
}

pub async fn get_workload(
    State(state): State<Arc<RwLock<ControllerState>>>,
    Path(workload_id): Path<WorkloadId>,
) -> Result<impl IntoResponse> {
    let workload_details = {
        let state = state
            .read()
            .map_err(|_| ControllerError::PoisonError)?;
        
        state.get_workload(&workload_id)?
    };

    Ok(Json(workload_details))
}

pub async fn delete_workload(
    State(state): State<Arc<RwLock<ControllerState>>>,
    Path(workload_id): Path<WorkloadId>,
) -> Result<impl IntoResponse> {
    let (client, node) = {
        let state = state
            .read()
            .map_err(|_| ControllerError::PoisonError)?;

        let client = state
            .get_http_agent_client()
            .clone();

        let node = state
            .find_node_for_workload_id(workload_id.clone())
            .ok_or(ControllerError::NodeNotFound)?;

        (client, node)
    };

    client
        .delete_workload(&node.get_address(), workload_id.clone())
        .await?;
   
    tracing::info!("Workload deleted: {workload_id} -> {}", node.id);

    Ok(Json("Workload deleted"))
}

pub async fn update_nodes(
    State(state): State<Arc<RwLock<ControllerState>>>,
    Json(node): Json<Node>,
) -> Result<impl IntoResponse> {
    let node_id = node.id.clone();

    let mut state = state
        .write()
        .map_err(|_| ControllerError::PoisonError)?;

    state.update_nodes(node);

    tracing::info!("Node registered: {node_id}");

    Ok(Json("Node state updated"))
}

pub async fn get_nodes(
        State(state): State<Arc<RwLock<ControllerState>>>
    ) -> Result<impl IntoResponse> {
    let nodes = {
        let state = state
            .read()
            .map_err(|_| ControllerError::PoisonError)?;

        state.get_nodes()
    };

    Ok(Json(nodes))
}
