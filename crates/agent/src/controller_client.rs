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
            .post("http://localhost:3000/nodes") // temp hardcode
            .json(&node)
            .send()
            .await?;

        Ok(())
    }
}
