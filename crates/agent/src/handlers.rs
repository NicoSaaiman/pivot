use std::collections::HashMap;
use std::time::SystemTime;

use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};

use crate::{
    errors::{AgentError, Result},
    info, 
    metrics,
    state::AppState,
};

use crate::state::RuntimeHandle;

use shared::models::{
    ExecutionState, 
    Workload, 
    WorkloadId,
    WorkloadInfo,
    WorkloadStatus,
    WorkloadToDelete,
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
    let workload_info = {
        let mut state = state
            .agent
            .write()
            .map_err(|_| AgentError::PoisonError)?;

        let workload = state
            .workloads
            .get_mut(&workload_id)
            .ok_or_else(|| AgentError::WorkloadNotFound(workload_id.clone()))?;

        match workload {
            RuntimeHandle::Process(child) => {
                let execution_state = match child.try_wait() {
                    Ok(Some(_)) => ExecutionState::Exited,
                    Ok(None) => ExecutionState::Running,
                    Err(_) => ExecutionState::Unknown,
                };
                
                WorkloadInfo::Process(execution_state)
            }
            RuntimeHandle::Docker { container_id } => {
                WorkloadInfo::Docker(container_id.clone())
            }
            RuntimeHandle::Custom { pid } => {
                WorkloadInfo::Custom(*pid)
            }
        }
    };

    let execution_state = match workload_info {
        WorkloadInfo::Process(execution_state) => execution_state,
        WorkloadInfo::Docker(container_id) => {
            state.runtime.docker_get_state(&container_id).await?       
        }
        WorkloadInfo::Custom(pid) => {
            state.runtime.custom_get_state(pid)?        
        }
    };


    Ok(
        Json(WorkloadStatus {
            state: execution_state,
            last_updated: SystemTime::now(),
        })
    )
}

pub async fn get_workloads(
        State(state): State<AppState>,    
    ) -> Result<impl IntoResponse> {
    let mut workloads = HashMap::new();

    let workload_ids = {
        let agent_state = state
            .agent
            .read()
            .map_err(|_| AgentError::PoisonError)?;

        agent_state
            .workloads
            .keys()
            .cloned()
            .collect::<Vec<WorkloadId>>()
    };

    for workload_id in workload_ids {
        let workload_info = {
            let mut agent_state = state
                .agent
                .write()
                .map_err(|_| AgentError::PoisonError)?;

            let workload = agent_state
                .workloads
                .get_mut(&workload_id)
                .ok_or_else(|| AgentError::WorkloadNotFound(workload_id.clone()))?;

            match workload {
                RuntimeHandle::Process(child) => { 
                    let execution_state = match child.try_wait() {
                        Ok(Some(_)) => ExecutionState::Exited,
                        Ok(None) => ExecutionState::Running,
                        Err(_) => ExecutionState::Unknown,
                    };
                    
                    WorkloadInfo::Process(execution_state)
                }
                RuntimeHandle::Docker { container_id } => {
                    WorkloadInfo::Docker(container_id.clone())
                }
                RuntimeHandle::Custom { pid } => {
                    WorkloadInfo::Custom(*pid)
                }
            }
        };

        let execution_state = match workload_info {
            WorkloadInfo::Process(execution_state) => execution_state,
            WorkloadInfo::Docker(container_id) => {
                state.runtime.docker_get_state(&container_id).await?       
            }
            WorkloadInfo::Custom(pid) => {
                state.runtime.custom_get_state(pid)?
            }
        };

        workloads.insert(workload_id, execution_state);
    }

    Ok(Json(workloads))
}

pub async fn delete_workload(
    State(state): State<AppState>,
    Path(workload_id): Path<WorkloadId>,
) -> Result<impl IntoResponse> {
    let workload_to_delete = {
        let agent_state = state
            .agent
            .read()
            .map_err(|_| AgentError::PoisonError)?;

        let runtime_handle = agent_state
            .workloads
            .get(&workload_id)
            .ok_or_else(|| AgentError::WorkloadNotFound(workload_id.clone()))?; 

        match runtime_handle {
            RuntimeHandle::Process(_) => {
                WorkloadToDelete::Process
            }
            RuntimeHandle::Docker { container_id } => {
                WorkloadToDelete::Docker(container_id.clone())
            }
            RuntimeHandle::Custom { pid } => {
                WorkloadToDelete::Custom(*pid)
            }
        }
    };

    match workload_to_delete {
        WorkloadToDelete::Process => {
            let mut agent_state = state
                .agent
                .write()
                .map_err(|_| AgentError::PoisonError)?;

            let child = match agent_state.workloads.get_mut(&workload_id) {
                Some(RuntimeHandle::Process(child)) => child,
                _ => {
                    return Err(AgentError::WorkloadNotFound(workload_id));
                }
            };

            child.kill()?;

            agent_state.workloads.remove(&workload_id);

            Ok(Json("Process workload deleted"))
        }
        WorkloadToDelete::Docker(container_id) => {
            state
                .runtime
                .docker_delete_container(&container_id)
                .await?;

            let mut agent_state = state
                .agent
                .write()
                .map_err(|_| AgentError::PoisonError)?;

            agent_state.workloads.remove(&workload_id);

            Ok(Json("Docker workload deleted"))
        }
        WorkloadToDelete::Custom(pid) => {
            state
                .runtime
                .custom_terminate_process(pid)?;

            //let mut agent_state = state
            //    .agent
            //    .write()
            //    .map_err(|_| AgentError::PoisonError)?;

            //agent_state.workloads.remove(&workload_id);

            Ok(Json("Custom container deleted"))
        }
    }
}

pub async fn get_node_info() -> Result<impl IntoResponse> {
    let info = info::get_node_info()?;

    Ok(Json(info))
}

pub async fn get_node_metrics() -> Result<impl IntoResponse> {
    let metrics = metrics::get_node_metrics()?;

    Ok(Json(metrics))
}
