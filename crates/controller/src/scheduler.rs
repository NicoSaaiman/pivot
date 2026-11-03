use std::time::Instant;
use std::sync::{Arc, RwLock};

use shared::models::{
    Node,
    Workload,
};

use crate::agent_client::{AgentClient, HttpAgentClient};

use crate::{
    errors::{ControllerError, Result},
    state::{ControllerState, ScheduledWorkload},
};

const MAX_ATTEMPTS: u32 = 5; 

pub struct ScheduleManager {
    client: HttpAgentClient,
    state: Arc<RwLock<ControllerState>>,
    node: Node,
    workload: Workload,
}

impl ScheduleManager {
    pub fn build(state: Arc<RwLock<ControllerState>>, workload: Workload) -> Result<Self> {
        let (client, node) = {
            let state = state
                .read()
                .map_err(|_| ControllerError::PoisonError)?;

            let client = state
                .get_http_agent_client()
                .clone();

            // NB! snapshot of node
            let node = state
                .select_node()?
                .clone();

            (client, node)
        };

        Ok(
            Self {
                client,
                state,
                node,
                workload,
            }
        )
    }

    pub fn get_node(&self) -> &Node {
        &self.node
    }

    pub async fn schedule_workload(&self) -> Result<()> {
        self.record_desired_state()?;

        let mut node = self.node.clone();

        for attempt in 0..MAX_ATTEMPTS {
            match self.client
                .execute_workload(&node.get_address(), &self.workload)
                .await {
                    Ok(_) => return Ok(()),
                    Err(error) => {
                        let is_last_attempt = attempt + 1 == MAX_ATTEMPTS;

                        if is_last_attempt {
                            if matches!(
                                &error,
                                ControllerError::ReqwestError(reqwest_error)
                                    if reqwest_error.is_connect()
                            ) {
                                self.mark_node_offline(&node.id)?;
                            }

                            return Err(error);
                        }

                        node = self.handle_dispatch_errors(error, &node.id).await?; 
                    }
            };
        }

        unreachable!("MAX_ATTEMPTS must be greater than zero");
    }

    fn record_desired_state(&self) -> Result<()> {
        let mut state = self
            .state
            .write()
            .map_err(|_| ControllerError::PoisonError)?;

        state.update_desired_workloads(
            self.workload.get_workload_id(),
            ScheduledWorkload {
                workload: self.workload.clone(),
                assigned_node: Some(self.node.id.clone()),
                scheduled_at: Instant::now(),
            },
        );

        Ok(())
    }

    async fn handle_dispatch_errors(
            &self,
            error: ControllerError,
            failed_node_id: &str,
        ) -> Result<Node> {
        match &error {
            ControllerError::ReqwestError(reqwest_error) if reqwest_error.is_connect() => {
                // Connection failed: consider another node 
                {
                    let mut state = self
                        .state
                        .write()
                        .map_err(|_| ControllerError::PoisonError)?;

                    if let Some(managed_node) = state.nodes.get_mut(failed_node_id) {
                        managed_node.node.is_online = false;
                    }

                    let node = state.select_node()?.clone();
                
                    let desired_workload = state
                        .desired_workloads
                        .get_mut(&self.workload.get_workload_id())
                        .ok_or(ControllerError::WorkloadNotFound)?;

                    desired_workload.assigned_node = Some(node.id.clone());

                    Ok(node)
                }
            }
            ControllerError::ReqwestError(reqwest_error) if reqwest_error.is_timeout() => {
                // Timeout: Uncertain state
                Err(error)
            }
            ControllerError::ReqwestError(reqwest_error) if reqwest_error.is_status() => {
                // Non-success status code
                Err(error)
            }
            _ => { Err(error) }
        }

    }

    fn mark_node_offline(&self, node_id: &str) -> Result<()> {
        let mut state = self
            .state
            .write()
            .map_err(|_| ControllerError::PoisonError)?;

        if let Some(managed_node) = state.nodes.get_mut(node_id) {
            managed_node.node.is_online = false;
        }

        Ok(())
    }
}
