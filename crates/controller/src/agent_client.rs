use shared::models::{Workload, WorkloadId};

use crate::errors::Result;

#[allow(async_fn_in_trait)]
pub trait AgentClient {
    async fn execute_workload(&self, node_address: &str, workload: &Workload) -> Result<()>;
    async fn delete_workload(&self, node_address: &str, workload_id: WorkloadId) -> Result<()>;
}

#[derive(Clone)]
pub struct HttpAgentClient {
    pub client: reqwest::Client,
}

impl AgentClient for HttpAgentClient {
    async fn execute_workload(&self, node_address: &str, workload: &Workload) -> Result<()> {
        let url = format!("{node_address}/execute");
        self
            .client
            .post(&url)
            .json(workload)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }

    async fn delete_workload(&self, node_address: &str, workload_id: WorkloadId) -> Result<()> {
        let url = format!("{node_address}/workload/{workload_id}");
        self
            .client
            .delete(&url)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}
