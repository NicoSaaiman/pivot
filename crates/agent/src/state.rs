use std::collections::HashMap;
use std::process::Child;
use std::sync::{Arc, RwLock};

use shared::models::WorkloadId;

use crate::{
    controller_client::HttpControllerClient,
    runtime::RuntimeManager,
};

#[derive(Clone)]
pub struct AppState {
    pub agent: Arc<RwLock<AgentState>>,
    pub runtime: Arc<RuntimeManager>,
}

pub struct AgentState {
    controller_client: HttpControllerClient,
    pub workloads: HashMap<WorkloadId, RuntimeHandle>,
}

impl AgentState {
    pub fn new() -> Self {
        Self {
            controller_client: HttpControllerClient {
                client: reqwest::Client::new(),
            },
            workloads: HashMap::new(),
        }
    }

    pub fn get_http_controller_client(&self) -> &HttpControllerClient {
        &self.controller_client
    }

    pub fn add_workload(mut self, workload_id: WorkloadId, runtime_handle: RuntimeHandle) {
        self.workloads.insert(workload_id, runtime_handle);
    }
}

pub enum RuntimeHandle {
    Process(Child),
    Docker {
        container_id: String,
    },
    Custom,
}
