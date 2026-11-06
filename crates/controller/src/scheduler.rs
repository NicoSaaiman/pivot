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

#[cfg(test)]
mod tests {
    use super::*;

    use wiremock::ResponseTemplate;

    use test_utils;

    use shared::models::{
        ExecutionState,
    };

    #[test]
    fn build_schedule_manager() {
        let mut state = ControllerState::new();

        let node = test_utils::create_test_node("node1".to_string(), None);
        let workload = test_utils::create_test_workload();
        state.update_nodes(node);

        let controller_state = Arc::new(RwLock::new(state)); 

        let schedule_manager = ScheduleManager::build(
            controller_state,
            workload,
        ).unwrap();

        assert_eq!(schedule_manager.get_node().id, "node1");
    }

    #[test]
    fn record_desired_state() {
        let mut state = ControllerState::new();

        let node = test_utils::create_test_node("node1".to_string(), None);
        let workload = test_utils::create_test_workload();
        state.update_nodes(node);

        let controller_state = Arc::new(RwLock::new(state));

        let schedule_manager = ScheduleManager::build(
            controller_state.clone(),
            workload,
        ).unwrap();

        schedule_manager.record_desired_state().unwrap();

        {
            let controller_state = controller_state.read().unwrap();
            let scheduled = controller_state.desired_workloads.get("test-workload").unwrap();
            
            assert_eq!(
                scheduled.workload.get_desired_state(), 
                ExecutionState::Running
            ); 

            assert_eq!(
                scheduled.assigned_node.as_deref(), Some("node1")
            );
        }
    } 

    #[tokio::test]
    async fn handle_dispatch_errors() {
       let mut state = ControllerState::new();

        let node1 = test_utils::create_test_node("node1".to_string(), None);
        let node2 = test_utils::create_test_node("node2".to_string(), None);
        let workload = test_utils::create_test_workload();
        state.update_nodes(node1);
        state.update_nodes(node2);

        let controller_state = Arc::new(RwLock::new(state));

        let schedule_manager = ScheduleManager::build(
            controller_state.clone(),
            workload,
        ).unwrap();

        schedule_manager.record_desired_state().unwrap();

        let error = ControllerError::ReqwestError(
            reqwest::Client::new()
            .get("http://127.0.0.1:1")
            .send()
            .await
            .unwrap_err(),
        );
        
        let next_node = schedule_manager
            .handle_dispatch_errors(error, "node1")
            .await
            .unwrap();

        assert_eq!(next_node.id, "node2");

        let state = controller_state.read().unwrap();

        assert!(!state.nodes.get("node1").unwrap().node.is_online);
        assert!(state.nodes.get("node2").unwrap().node.is_online);

        let scheduled = state.desired_workloads.get("test-workload").unwrap();
        assert_eq!(scheduled.assigned_node.as_deref(), Some("node2"));
    }

    #[test]
    fn mark_node_offline() {
        let mut state = ControllerState::new();

        let node = test_utils::create_test_node("node1".to_string(), None);
        let workload = test_utils::create_test_workload();
        state.update_nodes(node);

        let controller_state = Arc::new(RwLock::new(state)); 

        let schedule_manager = ScheduleManager::build(
            controller_state.clone(),
            workload,
        ).unwrap();

        schedule_manager.mark_node_offline("node1").unwrap();
        {
            let state = controller_state.read().unwrap();

            let node = &state.nodes.get("node1").unwrap().node;
            assert!(!node.is_online);
        }
    }

    #[tokio::test]
    async fn schedule_workload() {
        let mock_server = test_utils::get_mock_server("POST", "/execute", ResponseTemplate::new(200)).await;

        let mut state = ControllerState::new();

        let node = test_utils::create_test_node("node1".to_string(), Some(mock_server.uri()));
        let workload = test_utils::create_test_workload();
        let workload_id = workload.get_workload_id();
        state.update_nodes(node);

        let controller_state = Arc::new(RwLock::new(state)); 

        let schedule_manager = ScheduleManager::build(
            controller_state.clone(),
            workload,
        ).unwrap();

        schedule_manager.schedule_workload().await.unwrap();

        {
            let state = controller_state.read().unwrap();

            let scheduled = state
                .desired_workloads
                .get(&workload_id)
                .unwrap();

            assert_eq!(scheduled.assigned_node.as_deref(), Some("node1"));
            assert_eq!(scheduled.workload.get_workload_id(), workload_id);
        }

        mock_server.verify().await;
    }
}
