use crate::component::*;
use crate::config::SidecarConfig;
use anyhow::{anyhow, Error};
use casper_event_sidecar::Database;
use std::process::ExitCode;
use tracing::info;

pub async fn run(config: SidecarConfig) -> Result<ExitCode, Error> {
    let maybe_database = if let Some(storage_config) = config.storage.as_ref() {
        Some(Database::build(storage_config).await?)
    } else {
        None
    };
    let mut components: Vec<Box<dyn Component>> = Vec::new();
    let admin_api_component = AdminApiComponent::new();
    components.push(Box::new(admin_api_component));
    let rest_api_component = RestApiComponent::new(maybe_database.clone());
    components.push(Box::new(rest_api_component));
    let sse_server_component = SseServerComponent::new(maybe_database);
    components.push(Box::new(sse_server_component));
    let rpc_api_component = RpcApiComponent::new();
    components.push(Box::new(rpc_api_component));
    do_run(config, components).await.map_err(|component_error| {
        info!("The server has exited with an error: {}", component_error);
        anyhow!(component_error.to_string())
    })
}

async fn do_run(
    config: SidecarConfig,
    components: Vec<Box<dyn Component>>,
) -> Result<ExitCode, ComponentError> {
    if components.is_empty() {
        info!("No sidecar components are defined/enabled. Exiting");
        return Ok(ExitCode::SUCCESS);
    }
    let mut component_futures = Vec::new();
    for component in components.iter() {
        let maybe_future = component.prepare_component_task(&config).await?;
        if let Some(future) = maybe_future {
            component_futures.push(future);
        }
    }
    futures::future::select_all(component_futures).await.0
}
