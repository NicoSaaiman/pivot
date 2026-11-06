use std::ffi::c_int;
use std::process::Command;
use std::sync::{Arc, OnceLock, RwLock};

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
    Workload,
};

use crate::{
    errors::{AgentError, Result},
    state::{AgentState, CustomProcessHandle, RuntimeHandle},
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
                let child = Command::new(&process.get_command())
                    .args(process.get_args())
                    .spawn()?;
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
                    let pid = match ffi::create_process(process) {
                        Ok(pid) => pid,
                        Err(error) => {
                            for process in &processes {
                                ffi::terminate_process(process.pid)?;
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
}
