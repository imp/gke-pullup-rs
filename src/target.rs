use anyhow::Context;
use gke::model::ReleaseChannel;
use gke::model::ServerConfig;
use gke::model::release_channel::Channel;

use super::*;

#[derive(Debug)]
pub enum Target {
    Channel(Channel),
    Version(String),
    Default,
}

impl Target {
    pub fn new(version: Option<String>, config: &ServerConfig) -> anyhow::Result<Self> {
        if let Some(version) = version {
            let channel = config
                .release_channel_by_name(&version)
                .map(|config| config.channel.clone());
            if let Some(channel) = channel {
                Ok(Self::Channel(channel))
            } else {
                if config.valid_master_versions.contains(&version) {
                    Ok(Self::Version(version))
                } else {
                    anyhow::bail!("Version {version} is not valid for this cluster")
                }
            }
        } else {
            Ok(Self::Default)
        }
    }

    pub fn find_compatible_version<'a>(
        &self,
        config: &'a ServerConfig,
        cluster_channel: ReleaseChannel,
    ) -> anyhow::Result<&'a str> {
        let version = match self {
            Self::Channel(channel) => {
                anyhow::ensure!(
                    cluster_channel.channel == *channel,
                    "Cluster is not subscribed to the requested release channel"
                );
                config
                    .release_channel_upgrade_target_version(&cluster_channel)
                    .unwrap_or(&config.default_cluster_version)
            }
            Self::Version(version) => config
                .release_channel_config(&cluster_channel)
                .map(|config| config.valid_versions.as_slice())
                .unwrap_or_default()
                .iter()
                .find(|v| *v == version)
                .context("Requested version is not valid for the cluster release channel")?,
            Self::Default => config
                .release_channel_upgrade_target_version(&cluster_channel)
                .unwrap_or(&config.default_cluster_version),
        };

        Ok(version)
    }
}
