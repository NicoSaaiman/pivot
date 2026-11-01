use std::process::Command;
use std::sync::{Arc, RwLock};

use bollard::Docker;
use bollard::models::ContainerStateStatusEnum;
use bollard::query_parameters::InspectContainerOptionsBuilder;
use bollard::query_parameters::CreateContainerOptionsBuilder;
use bollard::models::ContainerCreateBody;
use bollard::query_parameters::RemoveContainerOptionsBuilder;

use shared::models::{
    ExecutionState,
    ExecutionType,
    Workload,
};

use crate::{
    custom_container_ffi,
    errors::{AgentError, Result},
    state::{AgentState, RuntimeHandle},
};

pub struct RuntimeManager {
    state: Arc<RwLock<AgentState>>,
    docker: Docker,
}

impl RuntimeManager {
    pub fn build(state: Arc<RwLock<AgentState>>) -> Result<Self> {
        let docker = Docker::connect_with_local_defaults()?;  

        Ok(
            Self {
                state,
                docker,
            }
        )
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
                custom_container_ffi::print_process_details(&custom_container.process)?;
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
            .docker
            .create_container(Some(options), config)
            .await?;

        Ok(container.id)
    }

    pub async fn docker_start_container(&self, container_id: &str) -> Result<()> {
        self
            .docker
            .start_container(container_id, None)
            .await?;

        Ok(())
    }

    pub async fn docker_get_state(&self, container_id: &str) -> Result<ExecutionState> {
        let options = InspectContainerOptionsBuilder::default()
             .size(false)
             .build();

        let result = self
            .docker
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
            .docker
            .remove_container(container_id, Some(options))
            .await?;

        Ok(())
    }
}
