use std::collections::HashSet;
use std::sync::{Arc, RwLock};
use std::time::Duration;

use tokio::time;

use crate::{
    errors::{ControllerError, Result},
    state::ControllerState
};

use shared::models::{
    ExecutionState,
    WorkloadId,
};

const WORKLOAD_GRACE_PERIOD: Duration = Duration::from_secs(15);

pub async fn reconcile_workloads(state: Arc<RwLock<ControllerState>>) -> Result<()> {
    let mut converged_workloads = HashSet::new();
    let mut missing_workloads = HashSet::new();

    loop {
        reconcile_workloads_once(
            &state,
            &mut converged_workloads,
            &mut missing_workloads
        )?;
        
        time::sleep(Duration::from_secs(5)).await;
    }
}

fn reconcile_workloads_once(
    state: &Arc<RwLock<ControllerState>>,
    converged_workloads: &mut HashSet<WorkloadId>,
    missing_workloads: &mut HashSet<WorkloadId>,
) -> Result<()> {
    let state = state
        .read()
        .map_err(|_| ControllerError::PoisonError)?;

    for (workload_id, desired_workload) in state.desired_workloads.iter() {
        let desired_state = desired_workload.workload.get_desired_state();
        if let Some(actual_workload) = state.actual_workloads.get(workload_id) {
            missing_workloads.remove(workload_id);

            let actual_state = actual_workload.status.get_state();

            if desired_state == actual_state {
                if converged_workloads.insert(workload_id.clone()) {
                    tracing::info!("Workload reached desired state: {workload_id}");
                }
                continue;
            }

            converged_workloads.remove(workload_id);

            match (desired_state, actual_state) {
                (ExecutionState::Running, ExecutionState::Exited | ExecutionState::Stopped) => {
                    tracing::warn!("Restarting workload {workload_id}");
                }
                (ExecutionState::Running, ExecutionState::Unknown) => {
                    tracing::error!("Could not determine actual state for {workload_id}");
                }
                (_, _) => {}
            }
        } else {
            converged_workloads.remove(workload_id);

            if desired_workload.scheduled_at.elapsed() > WORKLOAD_GRACE_PERIOD {
                if missing_workloads.insert(workload_id.clone()) {
                    tracing::warn!("Workload missing from actual state: {workload_id}");
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    use std::time::Instant;

    use crate::state::{
        ActualWorkload,
        ScheduledWorkload,
        WorkloadStatus,
    };


    #[test]
    fn reconcile_workloads_converged() {
        let mut state = ControllerState::new();

        let workload = test_utils::create_test_workload();
        let workload_id = workload.get_workload_id();

        let scheduled_workload = ScheduledWorkload {
            workload,
            assigned_node: Some("node1".to_string()),
            scheduled_at: Instant::now(),
        }; 

        state.update_desired_workloads(workload_id.clone(), scheduled_workload);

        state
            .actual_workloads
            .insert(
                workload_id.clone(),
                ActualWorkload {
                    status: WorkloadStatus {
                        state: ExecutionState::Running,
                        last_updated: Instant::now(),
                    },
                    node: "node1".to_string(),
                },
            );

        let state = Arc::new(RwLock::new(state));

        let mut converged_workloads = HashSet::new();
        let mut missing_workloads = HashSet::new();

        reconcile_workloads_once(
            &state,
            &mut converged_workloads,
            &mut missing_workloads,
        ).unwrap();

        assert!(converged_workloads.contains(&workload_id));
        assert!(missing_workloads.is_empty());
    }

    #[test]
    fn reconcile_workloads_actual_exited() {
        let mut state = ControllerState::new();

        let workload = test_utils::create_test_workload();
        let workload_id = workload.get_workload_id();

        let scheduled_workload = ScheduledWorkload {
            workload,
            assigned_node: Some("node1".to_string()),
            scheduled_at: Instant::now(),
        }; 

        state.update_desired_workloads(workload_id.clone(), scheduled_workload);

        state
            .actual_workloads
            .insert(
                workload_id.clone(),
                ActualWorkload {
                    status: WorkloadStatus {
                        state: ExecutionState::Exited,
                        last_updated: Instant::now(),
                    },
                    node: "node1".to_string(),
                },
            );

        let state = Arc::new(RwLock::new(state));

        let mut converged_workloads = HashSet::new();
        converged_workloads.insert(workload_id.clone());
        let mut missing_workloads = HashSet::new();

        reconcile_workloads_once(
            &state,
            &mut converged_workloads,
            &mut missing_workloads,
        ).unwrap();

        assert!(!converged_workloads.contains(&workload_id));
        assert!(missing_workloads.is_empty());
    }

}
