use std::collections::HashMap;
use std::time::{Duration, Instant};

use serde::Serialize;

use shared::models::{ExecutionState, ManagedNode, Node, NodeId, Workload, WorkloadId};

use crate::agent_client::HttpAgentClient;
use crate::errors::{ControllerError, Result};

pub struct ControllerState {
    agent_client: HttpAgentClient,
    pub nodes: HashMap<NodeId, ManagedNode>,
    pub desired_workloads: HashMap<WorkloadId, ScheduledWorkload>,
    pub actual_workloads: HashMap<WorkloadId, ActualWorkload>,
}

impl ControllerState {
    pub fn new() -> Self {
        Self {
            agent_client: HttpAgentClient {
                client: reqwest::Client::new(),
            },
            nodes: HashMap::new(),
            desired_workloads: HashMap::new(),
            actual_workloads: HashMap::new(),
        }
    }

    pub fn get_http_agent_client(&self) -> &HttpAgentClient {
        &self.agent_client
    }

    pub fn get_nodes(&self) -> HashMap<NodeId, Node> {
        self.nodes
            .iter()
            .map(|(id, managed_node)| (id.clone(), managed_node.node.clone()))
            .collect()
    }

    pub fn select_node(&self) -> Result<Node> {
        self.nodes
            .values()
            .find(|managed_node| 
                managed_node.last_seen.elapsed() < Duration::from_secs(15) && 
                managed_node.node.is_online
            )
            .map(|managed_node| managed_node.node.clone())
            .ok_or(ControllerError::NoAvailableNodes)
    }

    pub fn find_node_for_workload_id(&self, workload_id: WorkloadId) -> Option<Node> {
        let scheduled_workload = self.desired_workloads.get(&workload_id)?;

        let node_id = scheduled_workload.assigned_node.as_ref()?;

        self.nodes
            .get(node_id)
            .map(|managed_node| managed_node.node.clone())
    }

    pub fn update_nodes(&mut self, node: Node) {
        self.nodes.insert(
            node.id.clone(),
            ManagedNode {
                node,
                last_seen: Instant::now(),
            },
        );
    }

    pub fn update_desired_workloads(
        &mut self,
        workload_id: WorkloadId,
        scheduled_workload: ScheduledWorkload,
    ) {
        self.desired_workloads
            .insert(workload_id, scheduled_workload);
    }

    pub fn get_workload(&self, workload_id: &WorkloadId) -> Result<WorkloadDetails> {
        let desired_workload = self
            .desired_workloads
            .get(workload_id)
            .ok_or(ControllerError::WorkloadNotFound)?;

        let actual_workload = self
            .actual_workloads
            .get(workload_id)
            .ok_or(ControllerError::WorkloadNotFound)?;

        Ok(WorkloadDetails {
            id: workload_id.clone(),
            desired_state: desired_workload.workload.get_desired_state(),
            actual_state: actual_workload.status.get_state(),
            node: desired_workload.assigned_node.clone(),
        })
    }
}

pub struct ScheduledWorkload {
    pub workload: Workload,
    pub assigned_node: Option<NodeId>,
    pub scheduled_at: Instant,
}

pub struct ActualWorkload {
    pub status: WorkloadStatus,
    pub node: NodeId,
}

pub struct WorkloadStatus {
    pub state: ExecutionState,
    pub last_updated: Instant,
}

impl WorkloadStatus {
    pub fn get_state(&self) -> ExecutionState {
        self.state.clone()
    }
}

#[derive(Serialize)]
pub struct WorkloadDetails {
    id: WorkloadId,
    desired_state: ExecutionState,
    actual_state: ExecutionState,
    node: Option<NodeId>,
}
