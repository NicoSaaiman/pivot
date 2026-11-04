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
    WorkloadStatus,
    WorkloadToDelete,
};

enum WorkloadStateSource {
    Resolved(ExecutionState),
    Docker(String),
}

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
                
                WorkloadStateSource::Resolved(execution_state)
            }
            RuntimeHandle::Docker { container_id } => {
                WorkloadStateSource::Docker(container_id.clone())
            }
            RuntimeHandle::Custom { processes } => {
                let mut process_states = Vec::with_capacity(processes.len());

                for process in processes {
                    if process.cached_state == ExecutionState::Running {
                        process.cached_state = state.runtime.custom_get_state(process.pid)?;
                    }
                    
                    process_states.push(process.cached_state.clone());
                }
                
                WorkloadStateSource::Resolved(aggregate_process_states(&process_states))
            }
        }
    };

    let execution_state = match workload_info {
        WorkloadStateSource::Resolved(execution_state) => execution_state,
        WorkloadStateSource::Docker(container_id) => {
            state.runtime.docker_get_state(&container_id).await?       
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
                    
                    WorkloadStateSource::Resolved(execution_state)
                }
                RuntimeHandle::Docker { container_id } => {
                    WorkloadStateSource::Docker(container_id.clone())
                }
                 RuntimeHandle::Custom { processes } => {
                    let mut process_states = Vec::with_capacity(processes.len());

                    for process in processes {
                        if process.cached_state == ExecutionState::Running {
                            process.cached_state = state.runtime.custom_get_state(process.pid)?;
                        }
                    
                        process_states.push(process.cached_state.clone());
                    }
                
                    WorkloadStateSource::Resolved(aggregate_process_states(&process_states))
                }
            }
        };

        let execution_state = match workload_info {
            WorkloadStateSource::Resolved(execution_state) => execution_state,
            WorkloadStateSource::Docker(container_id) => {
                state.runtime.docker_get_state(&container_id).await?       
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
            RuntimeHandle::Custom { processes } => {
                WorkloadToDelete::Custom(
                    processes.iter().map(|process| process.pid).collect()
                )
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
        WorkloadToDelete::Custom(pids) => {
            for pid in pids { // todo (handle unhappy cases)
                state.runtime.custom_terminate_process(pid)?;
            }

            Ok(Json("Custom container termination requested"))
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

fn aggregate_process_states(states: &[ExecutionState]) -> ExecutionState {
    if states.iter().all(|s| *s == ExecutionState::Running) {
        return ExecutionState::Running;
    }

    if states.contains(&ExecutionState::Exited) {
        return ExecutionState::Exited;
    }

    if states.contains(&ExecutionState::Unknown) {
        return ExecutionState::Unknown;
    }

    if states.iter().all(|s| *s == ExecutionState::Stopped) {
        return ExecutionState::Stopped;
    }

    ExecutionState::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    use shared::models::ExecutionState;

    #[test]
    fn aggregate_all_running_processes() {
        let states = vec! [
            ExecutionState::Running,
            ExecutionState::Running
        ];

        let aggregate_state = aggregate_process_states(&states);
        assert_eq!(aggregate_state, ExecutionState::Running);
    }
}
