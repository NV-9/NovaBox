pub mod monitor;

use anyhow::Result;
use bollard::Docker;

pub async fn init() -> Result<Docker> {
    let docker = Docker::connect_with_socket_defaults()?;
    docker.ping().await?;
    tracing::info!("Docker connection established");
    Ok(docker)
}

pub async fn resolve_servers_host_path(docker: &Docker) -> String {
    let fallback = std::env::var("SERVERS_HOST_PATH")
        .unwrap_or_else(|_| "/opt/novabox/servers".to_string());

    let own_id = container_id();

    if let Some(cid) = own_id {
        if let Ok(info) = docker.inspect_container(&cid, None::<bollard::container::InspectContainerOptions>).await {
            if let Some(mounts) = info.mounts {
                for mount in &mounts {
                    if mount.destination.as_deref() == Some("/servers") {
                        if let Some(src) = &mount.source {
                            tracing::info!("Resolved servers host path from container mounts: {}", src);
                            return src.clone();
                        }
                    }
                }
            }
        }
    }

    tracing::info!("Could not resolve servers host path from mounts, using SERVERS_HOST_PATH={}", fallback);
    fallback
}

fn container_id() -> Option<String> {
    if let Ok(hostname) = std::env::var("HOSTNAME") {
        let trimmed = hostname.trim();
        if trimmed.len() >= 12 && trimmed.chars().all(|c| c.is_ascii_hexdigit()) {
            return Some(trimmed.to_string());
        }
    }

    match std::fs::read_to_string("/proc/self/cgroup") {
        Ok(cgroup) => cgroup
            .lines()
            .filter_map(|l| l.rsplit('/').next().map(|s| s.trim().to_string()))
            .find(|s| s.len() >= 12 && s.chars().all(|c| c.is_ascii_hexdigit())),
        Err(_) => None,
    }
}

pub fn container_name(server_id: &str) -> String {
    format!("novabox-mc-{}", &server_id[..8])
}
