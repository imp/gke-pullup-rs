use anyhow::Context as _;
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

#[cfg(test)]
mod tests {
    use super::*;
    use gke::model::server_config::ReleaseChannelConfig;

    fn make_server_config() -> ServerConfig {
        let channel_config = ReleaseChannelConfig::default()
            .set_channel(Channel::Regular)
            .set_upgrade_target_version("1.31.0")
            .set_valid_versions(["1.31.0", "1.30.0"]);

        ServerConfig::default()
            .set_default_cluster_version("1.30.0")
            .set_valid_master_versions(["1.30.0", "1.31.0"])
            .set_channels([channel_config])
    }

    #[test]
    fn new_with_none_returns_default() {
        let config = make_server_config();
        let target = Target::new(None, &config).unwrap();
        assert!(matches!(target, Target::Default));
    }

    #[test]
    fn new_with_channel_name_returns_channel_variant() {
        let config = make_server_config();
        let target = Target::new(Some("REGULAR".to_string()), &config).unwrap();
        assert!(matches!(target, Target::Channel(Channel::Regular)));
    }

    #[test]
    fn new_with_valid_version_returns_version_variant() {
        let config = make_server_config();
        let target = Target::new(Some("1.31.0".to_string()), &config).unwrap();
        assert!(matches!(target, Target::Version(v) if v == "1.31.0"));
    }

    #[test]
    fn new_with_invalid_version_is_error() {
        let config = make_server_config();
        assert!(Target::new(Some("99.99.99".to_string()), &config).is_err());
    }

    #[test]
    fn find_compatible_version_default_returns_upgrade_target() {
        let config = make_server_config();
        let cluster_channel = ReleaseChannel::new().set_channel(Channel::Regular);
        let version = Target::Default
            .find_compatible_version(&config, cluster_channel)
            .unwrap();
        assert_eq!(version, "1.31.0");
    }

    #[test]
    fn find_compatible_version_default_falls_back_to_default_cluster_version() {
        let config = ServerConfig::default().set_default_cluster_version("1.30.0");
        // No channel config → falls back to default_cluster_version
        let cluster_channel = ReleaseChannel::new().set_channel(Channel::Regular);
        let version = Target::Default
            .find_compatible_version(&config, cluster_channel)
            .unwrap();
        assert_eq!(version, "1.30.0");
    }

    #[test]
    fn find_compatible_version_channel_match_returns_upgrade_target() {
        let config = make_server_config();
        let cluster_channel = ReleaseChannel::new().set_channel(Channel::Regular);
        let version = Target::Channel(Channel::Regular)
            .find_compatible_version(&config, cluster_channel)
            .unwrap();
        assert_eq!(version, "1.31.0");
    }

    #[test]
    fn find_compatible_version_channel_mismatch_is_error() {
        let config = make_server_config();
        let cluster_channel = ReleaseChannel::new().set_channel(Channel::Rapid);
        let result =
            Target::Channel(Channel::Regular).find_compatible_version(&config, cluster_channel);
        assert!(result.is_err());
    }

    #[test]
    fn find_compatible_version_explicit_version_in_channel() {
        let config = make_server_config();
        let cluster_channel = ReleaseChannel::new().set_channel(Channel::Regular);
        let version = Target::Version("1.30.0".to_string())
            .find_compatible_version(&config, cluster_channel)
            .unwrap();
        assert_eq!(version, "1.30.0");
    }

    #[test]
    fn find_compatible_version_version_not_in_channel_is_error() {
        let config = make_server_config();
        let cluster_channel = ReleaseChannel::new().set_channel(Channel::Regular);
        let result = Target::Version("99.99.99".to_string())
            .find_compatible_version(&config, cluster_channel);
        assert!(result.is_err());
    }
}
