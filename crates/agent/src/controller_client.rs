use shared::models::Node;

use crate::{
    errors::Result,
    info, 
    metrics,
};

#[allow(async_fn_in_trait)]
pub trait ControllerClient {
    async fn register_node(&self) -> Result<()>;
}

#[derive(Clone)]
pub struct HttpControllerClient {
    pub client: reqwest::Client,
    pub controller_url: String,
}

impl ControllerClient for HttpControllerClient {
    async fn register_node(&self) -> Result<()> {
        let info = info::get_node_info()?;
        let metrics = metrics::get_node_metrics()?;

        let id = info.node_id.clone();

        let node = Node {
            id,
            info,
            metrics,
            is_online: true,
        };

        self.client
            .post(format!("{}/nodes", self.controller_url))
            .json(&node)
            .send()
            .await?
            .error_for_status()?;

        Ok(())
    }
}
