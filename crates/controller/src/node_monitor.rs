use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use std::time::{Duration, Instant};

use shared::models::{ExecutionState, WorkloadId};

use crate::{
    errors::{ControllerError, Result},
    state::{ActualWorkload, ControllerState, WorkloadStatus}
};

pub async fn poll_nodes(state: Arc<RwLock<ControllerState>>) -> Result<()> {
    let client = reqwest::Client::new();

    loop {
        let nodes = {
            let state = state
                .read()
                .map_err(|_| ControllerError::PoisonError)?;

            state.get_nodes()
        };

        for (node_id, node) in nodes {
            let address = node.get_address();
            let url = format!("{address}/workloads");
            let response = match client.get(&url).send().await {
                Ok(response) => response,
                Err(error) => {
                    println!("Failed to contact node {node_id}: {error}");
                    continue;
                }
            };

            let response = match response.error_for_status() {
                Ok(response) => response,
                Err(error) => {
                    println!("Node {node_id} returned an error: {error}");
                    continue;
                }
            };

            let workloads: HashMap<WorkloadId, ExecutionState> = match response.json().await {
                Ok(workloads) => workloads,
                Err(error) => {
                    println!("Failed to read workloads from {node_id}: {error}");
                    continue;
                }
            };

            {
                let mut state = state
                    .write()
                    .map_err(|_| ControllerError::PoisonError)?;

                if let Some(managed_node) = state.nodes.get_mut(&node_id) {
                    managed_node.last_seen = Instant::now();
                    managed_node.node.is_online = true;
                }

                for (workload_id, status) in workloads {
                    let workload_status = WorkloadStatus {
                        state: status,
                        last_updated: Instant::now(),
                    };

                    let actual_workload = ActualWorkload {
                        status: workload_status,
                        node: node_id.clone(),
                    };

                    state.actual_workloads.insert(workload_id, actual_workload);
                }
            }
        }

        {
            let mut state = state
                .write()
                .map_err(|_| ControllerError::PoisonError)?;

            for managed_node in state.nodes.values_mut() {
                if managed_node.last_seen.elapsed() > Duration::from_secs(15) {
                    managed_node.node.is_online = false;
                }
            }
        }

        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
