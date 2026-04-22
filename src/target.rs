use gke::model::ReleaseChannel;
use gke::model::ServerConfig;
use gke::model::release_channel::Channel;

use super::*;

#[derive(Debug)]
pub enum Target {
    Channel {
        config: ServerConfig,
        channel: Channel,
    },
    Version {
        config: ServerConfig,
        version: String,
    },
    Default {
        config: ServerConfig,
    },
}

impl Target {
    pub fn new(version: Option<String>, config: ServerConfig) -> anyhow::Result<Self> {
        if let Some(version) = version {
            let channel = config
                .release_channel_by_name(&version)
                .map(|config| config.channel.clone());
            if let Some(channel) = channel {
                Ok(Self::Channel { config, channel })
            } else {
                if config.valid_master_versions.contains(&version) {
                    Ok(Self::Version { config, version })
                } else {
                    anyhow::bail!("Version {version} is not valid for this cluster")
                }
            }
        } else {
            Ok(Self::Default { config })
        }
    }

    pub fn find_compatible_version(&self, cluster_channel: ReleaseChannel) -> anyhow::Result<&str> {
        let version = match self {
            Self::Channel { channel, config } => {
                anyhow::ensure!(
                    cluster_channel.channel == *channel,
                    "Cluster is not in the requested release channel"
                );
                config
                    .release_channel_upgrade_target_version(&cluster_channel)
                    .unwrap_or(&config.default_cluster_version)
            }
            Self::Version { version, config } => {
                let valid_versions = config
                    .release_channel_config(&cluster_channel)
                    .map(|config| config.valid_versions.as_slice())
                    .unwrap_or_default();
                anyhow::ensure!(
                    valid_versions.contains(version),
                    "Requested version is not valid for the cluster release channel"
                );
                version
            }
            Self::Default { config } => config
                .release_channel_upgrade_target_version(&cluster_channel)
                .unwrap_or(&config.default_cluster_version),
        };

        Ok(version)
    }

    pub fn short(&self) -> &str {
        match self {
            Self::Channel { channel, .. } => channel.name().unwrap_or_default(),
            Self::Version { version, .. } => version.as_str(),
            Self::Default { .. } => "default",
        }
    }
}
