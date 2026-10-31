use std::{env, fs};

use shared::models::NodeInfo;

use crate::errors::Result;

pub fn get_node_info() -> Result<NodeInfo> {
    let node_id = env::args()
        .nth(1)
        .unwrap_or("default".to_string());

    let port = env::args()
        .nth(2)
        .unwrap_or("3000".to_string());

    let info = NodeInfo {
        address: format!("http://localhost:{port}"),
        hostname: get_hostname()?,
        machine_id: get_machine_id()?,
        node_id,
        port,
    };

    Ok(info)
}

pub fn get_hostname() -> Result<String> {
    Ok(fs::read_to_string("/etc/hostname")?)
}

pub fn get_machine_id() -> Result<String> {
    Ok(fs::read_to_string("/etc/machine-id")?)
}
