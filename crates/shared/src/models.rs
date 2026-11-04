use std::collections::BTreeMap;
use std::ffi::c_int;
use std::time::{Instant, SystemTime};

use serde::{Deserialize, Serialize};

pub type NodeId = String;
pub type WorkloadId = String;
type GroupId = String;

//--------------Workloads----------------

#[derive(Clone, Serialize, Deserialize)]
pub struct Workload {
    id: WorkloadId,
    group_id: GroupId,
    desired_state: ExecutionState,
    execution_type: ExecutionType,
}

impl Workload {
    pub fn get_workload_id(&self) -> WorkloadId {
        self.id.clone()
    }

    pub fn get_execution_type(&self) -> ExecutionType {
        self.execution_type.clone()
    }

    pub fn get_desired_state(&self) -> ExecutionState {
        self.desired_state.clone()
    }
}

#[derive(Serialize)]
pub struct WorkloadStatus {
    pub state: ExecutionState,
    pub last_updated: SystemTime,
}

pub enum WorkloadInfo {
    Process(ExecutionState),
    Docker(String),
    Custom(c_int),
}

pub enum WorkloadToDelete {
    Process,
    Docker(String),
    Custom(Vec<c_int>),
}

//--------------Nodes----------------

#[derive(Clone, Serialize, Deserialize)]
pub struct Node {
    pub id: NodeId,
    pub info: NodeInfo,
    pub metrics: NodeMetrics,
    pub is_online: bool,
}

impl Node {
    pub fn get_address(&self) -> String {
        self.info.address.clone()
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    pub address: String,
    pub hostname: String,
    pub machine_id: String,
    pub node_id: String,
    pub port: String,
}

impl NodeInfo {
    pub fn collect() -> Self {
        let node_id = std::env::args().nth(1).unwrap_or("default".to_string());
        let port = std::env::args().nth(2).unwrap_or("3000".to_string());

        Self {
            address: "".to_string(),
            hostname: "".to_string(),
            machine_id: "".to_string(),
            node_id,
            port,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct NodeMetrics {
    pub cpu_usage: f64,
    pub memory_usage: MemoryUsage,
    pub load_average: LoadAverage,
    pub uptime: f64,
}

pub struct CpuUsage {
    pub busy: f64,
    pub total: f64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct MemoryUsage {
    pub used_kb: f64,
    pub total_kb: f64,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct LoadAverage {
    pub min_1: f64,
    pub min_5: f64,
    pub min_15: f64,
}

pub struct ManagedNode {
    pub node: Node,
    pub last_seen: Instant,
}

#[derive(Clone, Serialize, Deserialize, PartialEq, Debug)]
pub enum ExecutionState {
    Running,
    Stopped,
    Exited,
    Unknown,
}

//--------------Runtime----------------

#[derive(Clone, Serialize, Deserialize)]
pub enum ExecutionType {
    Process(Process),
    DockerContainer(DockerContainer),
    CustomContainer(CustomContainer),
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CustomContainer {
    pub processes: Vec<Process>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct DockerContainer {
    pub name: String,
    pub image: Option<String>,
    pub cmd: Option<Vec<String>>,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Process {
    command: String,
    args: Vec<String>,
    env: BTreeMap<String, String>,
    working_directory: Option<String>,
}

impl Process {
    pub fn get_command(&self) -> &str {
        &self.command
    }

    pub fn get_args(&self) -> &[String] {
        &self.args
    }

    pub fn get_env(&self) -> &BTreeMap<String, String> {
        &self.env
    }

    pub fn get_working_dir(&self) -> Option<&str> {
        self.working_directory.as_deref()
    }
}
