use std::collections::HashMap;
use std::ffi::c_int;
use std::process::{Child, Command};
use std::sync::{Arc, OnceLock, RwLock};
use std::time::SystemTime;

use bollard::Docker;
use bollard::models::ContainerStateStatusEnum;
use bollard::query_parameters::InspectContainerOptionsBuilder;
use bollard::query_parameters::CreateContainerOptionsBuilder;
use bollard::models::ContainerCreateBody;
use bollard::query_parameters::RemoveContainerOptionsBuilder;

use runtime::ffi;

use shared::models::{
    ExecutionState,
    ExecutionType,
    Process,
    Workload,
    WorkloadId,
    WorkloadStatus,
    WorkloadToDelete,
};

use crate::{
    errors::{AgentError, Result},
    state::{
        AgentState, 
        CustomProcessHandle, 
        RuntimeHandle,
    },
};

pub struct RuntimeManager {
    state: Arc<RwLock<AgentState>>,
    docker: OnceLock<Docker>,
}

impl RuntimeManager {
    pub fn build(state: Arc<RwLock<AgentState>>) -> Self {
        Self {
            state,
            docker: OnceLock::new(),
        }
    }

    pub fn docker(&self) -> Result<&Docker> {
        if self.docker.get().is_none() {
            let docker = Docker::connect_with_local_defaults()?;
            let _ = self.docker.set(docker);
        }

        Ok(self.docker.get().expect("Docker client was initialized"))
    }

    pub async fn execute_workload(&self, workload: Workload) -> Result<()> {
        match workload.get_execution_type() {
            ExecutionType::Process(process) => {
                let child = self.process_create(&process)?;

                {
                    let mut state = self
                        .state
                        .write()
                        .map_err(|_| AgentError::PoisonError)?;

                    state
                        .workloads
                        .insert(workload.get_workload_id(), RuntimeHandle::Process(child));
                }
            }
            ExecutionType::DockerContainer(docker_container) => {
                let container_id = self
                    .docker_create_container(
                        &docker_container.name, 
                        docker_container.image, 
                        docker_container.cmd,
                    )
                    .await?;
                
                self
                    .docker_start_container(&container_id)
                    .await?;

                let mut state = self
                    .state
                    .write()
                    .map_err(|_| AgentError::PoisonError)?;

                state
                    .workloads
                    .insert(workload.get_workload_id(), RuntimeHandle::Docker { container_id });
            },
            ExecutionType::CustomContainer(custom_container) => {
                if custom_container.processes.is_empty() {
                    return Err(AgentError::EmptyProcessCollection);
                }

                let mut processes: Vec<CustomProcessHandle> = Vec::with_capacity(custom_container.processes.len());

                for process in &custom_container.processes {
                    let pid = match self.custom_create_process(process) {
                        Ok(pid) => pid,
                        Err(error) => {
                            for process in &processes {
                                self.custom_terminate_process(process.pid)?;
                            }

                            return Err(error.into());
                        }
                    };

                    processes.push(CustomProcessHandle {
                        pid,
                        cached_state: ExecutionState::Running,
                    });
                }

                let mut state = self
                    .state
                    .write()
                    .map_err(|_| AgentError::PoisonError)?;

                state
                    .workloads
                    .insert(workload.get_workload_id(), RuntimeHandle::Custom { processes });

            },
        }

        Ok(())
    }

    pub async fn get_workload(&self, workload_id: WorkloadId) -> Result<WorkloadStatus> {
        let workload_info = {
            let mut agent_state = self
                .state
                .write()
                .map_err(|_| AgentError::PoisonError)?;

            let workload = agent_state
                .workloads
                .get_mut(&workload_id)
                .ok_or_else(|| AgentError::WorkloadNotFound(workload_id.clone()))?;

            match workload {
                RuntimeHandle::Process(child) => {
                    let execution_state = match child.try_wait() {
                        Ok(Some(_)) => ExecutionState::Exited,
                        Ok(None) => ExecutionState::Running,
                        Err(_) => ExecutionState::Unknown,
                    };
                    
                    WorkloadStateSource::Resolved(execution_state)
                }
                RuntimeHandle::Docker { container_id } => {
                    WorkloadStateSource::Docker(container_id.clone())
                }
                RuntimeHandle::Custom { processes } => {
                    let mut process_states = Vec::with_capacity(processes.len());

                    for process in processes {
                        if process.cached_state == ExecutionState::Running {
                            process.cached_state = self.custom_get_state(process.pid)?;
                        }
                        
                        process_states.push(process.cached_state.clone());
                    }
                    
                    WorkloadStateSource::Resolved(aggregate_process_states(&process_states))
                }
            }
        };

        let execution_state = match workload_info {
            WorkloadStateSource::Resolved(execution_state) => execution_state,
            WorkloadStateSource::Docker(container_id) => {
                self.docker_get_state(&container_id).await?       
            }
        };
        
        let workload_status = WorkloadStatus {
            state: execution_state,
            last_updated: SystemTime::now(),
        };

        Ok(workload_status)
    }

    pub async fn get_workloads(&self) -> Result<HashMap<WorkloadId, ExecutionState>> {
            let mut workloads = HashMap::new();

        let workload_ids = {
            let agent_state = self
                .state
                .read()
                .map_err(|_| AgentError::PoisonError)?;

            agent_state
                .workloads
                .keys()
                .cloned()
                .collect::<Vec<WorkloadId>>()
        };

        for workload_id in workload_ids {
            let workload_info = {
                let mut agent_state = self
                    .state
                    .write()
                    .map_err(|_| AgentError::PoisonError)?;

                let workload = agent_state
                    .workloads
                    .get_mut(&workload_id)
                    .ok_or_else(|| AgentError::WorkloadNotFound(workload_id.clone()))?;

                match workload {
                    RuntimeHandle::Process(child) => { 
                        let execution_state = match child.try_wait() {
                            Ok(Some(_)) => ExecutionState::Exited,
                            Ok(None) => ExecutionState::Running,
                            Err(_) => ExecutionState::Unknown,
                        };
                        
                        WorkloadStateSource::Resolved(execution_state)
                    }
                    RuntimeHandle::Docker { container_id } => {
                        WorkloadStateSource::Docker(container_id.clone())
                    }
                     RuntimeHandle::Custom { processes } => {
                        let mut process_states = Vec::with_capacity(processes.len());

                        for process in processes {
                            if process.cached_state == ExecutionState::Running {
                                process.cached_state = self.custom_get_state(process.pid)?;
                            }
                        
                            process_states.push(process.cached_state.clone());
                        }
                    
                        WorkloadStateSource::Resolved(aggregate_process_states(&process_states))
                    }
                }
            };

            let execution_state = match workload_info {
                WorkloadStateSource::Resolved(execution_state) => execution_state,
                WorkloadStateSource::Docker(container_id) => {
                    self.docker_get_state(&container_id).await?       
                }
            };

            workloads.insert(workload_id, execution_state);
        }

        Ok(workloads)
    }

    pub async fn delete_workload(&self, workload_id: WorkloadId) -> Result<String> {
        let workload_to_delete = {
            let agent_state = self
                .state
                .read()
                .map_err(|_| AgentError::PoisonError)?;

            let runtime_handle = agent_state
                .workloads
                .get(&workload_id)
                .ok_or_else(|| AgentError::WorkloadNotFound(workload_id.clone()))?; 

            match runtime_handle {
                RuntimeHandle::Process(_) => {
                    WorkloadToDelete::Process
                }
                RuntimeHandle::Docker { container_id } => {
                    WorkloadToDelete::Docker(container_id.clone())
                }
                RuntimeHandle::Custom { processes } => {
                    WorkloadToDelete::Custom(
                        processes.iter().map(|process| process.pid).collect()
                    )
                }
            }
        };

        match workload_to_delete {
            WorkloadToDelete::Process => {
                let mut agent_state = self
                    .state
                    .write()
                    .map_err(|_| AgentError::PoisonError)?;

                let child = match agent_state.workloads.get_mut(&workload_id) {
                    Some(RuntimeHandle::Process(child)) => child,
                    _ => {
                        return Err(AgentError::WorkloadNotFound(workload_id));
                    }
                };

                child.kill()?;
                child.wait()?;

                agent_state.workloads.remove(&workload_id);

                Ok("Process workload deleted".to_string())
            }
            WorkloadToDelete::Docker(container_id) => {
                self
                    .docker_delete_container(&container_id)
                    .await?;

                let mut agent_state = self
                    .state
                    .write()
                    .map_err(|_| AgentError::PoisonError)?;

                agent_state.workloads.remove(&workload_id);

                Ok("Docker workload deleted".to_string())
            }
            WorkloadToDelete::Custom(pids) => {
                for pid in pids { // todo (handle unhappy cases)
                    self.custom_terminate_process(pid)?;
                }

                Ok("Custom container termination requested".to_string())
            }
        }
    }

    pub async fn docker_create_container(&self, name: &str, image: Option<String>, cmd: Option<Vec<String>>) -> Result<String> {
        let options = CreateContainerOptionsBuilder::default()
            .name(name)
            .build();
    
        let config = ContainerCreateBody {
            image,
            cmd,
            ..Default::default()
        };

        let container = self
            .docker()?
            .create_container(Some(options), config)
            .await?;

        Ok(container.id)
    }

    pub async fn docker_start_container(&self, container_id: &str) -> Result<()> {
        self
            .docker()?
            .start_container(container_id, None)
            .await?;

        Ok(())
    }

    pub async fn docker_get_state(&self, container_id: &str) -> Result<ExecutionState> {
        let options = InspectContainerOptionsBuilder::default()
             .size(false)
             .build();

        let result = self
            .docker()?
            .inspect_container(container_id, Some(options))
            .await?;

        let state = result
            .state
            .ok_or(AgentError::UnknownExecutionState)?;

        let status = state
            .status
            .ok_or(AgentError::UnknownExecutionStatus)?;

        let state = match status {
            ContainerStateStatusEnum::RUNNING => ExecutionState::Running,
            ContainerStateStatusEnum::EXITED => ExecutionState::Exited,
            _ => ExecutionState::Unknown
        };

        Ok(state)
    }

    pub async fn docker_delete_container(&self, container_id: &str) -> Result<()> {
        let options = RemoveContainerOptionsBuilder::default()
            .force(true)
            .build();

        self
            .docker()?
            .remove_container(container_id, Some(options))
            .await?;

        Ok(())
    }

    pub fn custom_get_state(&self, pid: c_int) -> Result<ExecutionState> {
        let state = ffi::get_process_state(pid)?;
        Ok(state)
    }

    pub fn custom_terminate_process(&self, pid: c_int) -> Result<()> {
        ffi::terminate_process(pid)?;
        Ok(())
    }
    
    pub fn custom_wait_process_blocking(&self, pid: c_int) -> Result<()> {
        ffi::wait_process_blocking(pid)?;
        Ok(())
    }

    pub fn custom_create_process(&self, process: &Process) -> Result<c_int> {
        let pid = ffi::create_process(process)?;
        Ok(pid)
    }

    pub fn process_create(&self, process: &Process) -> Result<Child> {
        let child = Command::new(&process.get_command())
            .args(process.get_args())
            .spawn()?;
        
        Ok(child)
    }
}

enum WorkloadStateSource {
    Resolved(ExecutionState),
    Docker(String),
}

fn aggregate_process_states(states: &[ExecutionState]) -> ExecutionState {
    if states.iter().all(|s| *s == ExecutionState::Running) {
        return ExecutionState::Running;
    }

    if states.contains(&ExecutionState::Exited) {
        return ExecutionState::Exited;
    }

    if states.contains(&ExecutionState::Unknown) {
        return ExecutionState::Unknown;
    }

    if states.iter().all(|s| *s == ExecutionState::Stopped) {
        return ExecutionState::Stopped;
    }

    ExecutionState::Unknown
}

#[cfg(test)]
mod tests {
    use super::*;

    use shared::models::ExecutionState;

    #[test]
    fn aggregate_all_running_processes() {
        let states = vec! [
            ExecutionState::Running,
            ExecutionState::Running
        ];

        let aggregate_state = aggregate_process_states(&states);
        assert_eq!(aggregate_state, ExecutionState::Running);
    }

    #[test]
    fn aggregate_contains_exited_process() {
        let states = vec! [
            ExecutionState::Running,
            ExecutionState::Exited
        ];

        let aggregate_state = aggregate_process_states(&states);
        assert_eq!(aggregate_state, ExecutionState::Exited);
    }

    #[test]
    fn aggregate_contains_unknown_process() {
        let states = vec! [
            ExecutionState::Running,
            ExecutionState::Unknown
        ];

        let aggregate_state = aggregate_process_states(&states);
        assert_eq!(aggregate_state, ExecutionState::Unknown);
    }

    #[test]
    fn aggregate_all_stopped_processes() {
        let states = vec! [
            ExecutionState::Stopped,
            ExecutionState::Stopped
        ];

        let aggregate_state = aggregate_process_states(&states);
        assert_eq!(aggregate_state, ExecutionState::Stopped);
    }
}
