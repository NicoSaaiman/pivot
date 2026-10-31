use std::sync::{Arc, RwLock};
use std::time::Duration;

use crate::{
    errors::{ControllerError, Result},
    state::ControllerState
};

use shared::models::ExecutionState;

pub async fn reconcile_workloads(state: Arc<RwLock<ControllerState>>) -> Result<()> {
    loop {
        {
            let state = state
                .read()
                .map_err(|_| ControllerError::PoisonError)?;

            println!("Desired workloads: {}", state.desired_workloads.len());
            println!("Actual workloads: {}", state.actual_workloads.len());

            for (workload_id, desired_workload) in state.desired_workloads.iter() {
                let desired_state = desired_workload.workload.get_desired_state();
                if let Some(actual_workload) = state.actual_workloads.get(workload_id) {
                    let actual_state = actual_workload.status.get_state();

                    if desired_state == actual_state {
                        println!("No issues found with {workload_id}");
                        continue;
                    }

                    match (desired_state, actual_state) {
                        (ExecutionState::Running, ExecutionState::Exited | ExecutionState::Stopped) => {
                            println!("Restarting workload {workload_id}");
                        }
                        (ExecutionState::Running, ExecutionState::Unknown) => {
                            println!("Could not determine actual state for {workload_id}");
                        }
                        (_, _) => {}
                    }
                } else {
                    println!("Workload is missing from actual state!");
                }
            }
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
