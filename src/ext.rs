use gke::model::Cluster;
use gke::model::ReleaseChannel;
use gke::model::ServerConfig;
use gke::model::release_channel::Channel;
use gke::model::server_config::ReleaseChannelConfig;

use super::*;

pub trait ClusterExt {
    fn release_channel(&self) -> Option<&ReleaseChannel>;

    fn release_channel_or_regular(&self) -> ReleaseChannel {
        self.release_channel()
            .cloned()
            .unwrap_or_else(|| ReleaseChannel::new().set_channel(Channel::Regular))
    }

    fn release_channel_name(&self) -> Option<&str> {
        self.release_channel()?.channel.name()
    }
}

impl ClusterExt for Cluster {
    fn release_channel(&self) -> Option<&ReleaseChannel> {
        self.release_channel.as_ref()
    }
}

pub trait ServerConfigExt {
    fn release_channel_by_name(&self, name: &str) -> Option<&ReleaseChannelConfig>;

    fn release_channel_config(&self, channel: &ReleaseChannel) -> Option<&ReleaseChannelConfig>;

    fn release_channel_upgrade_target_version(&self, channel: &ReleaseChannel) -> Option<&str> {
        self.release_channel_config(channel)
            .map(|config| config.upgrade_target_version.as_str())
    }

    fn show(&self);
}

impl ServerConfigExt for ServerConfig {
    fn release_channel_by_name(&self, name: &str) -> Option<&ReleaseChannelConfig> {
        self.channels
            .iter()
            .find(|config| config.channel.name() == Some(name))
    }

    fn release_channel_config(&self, channel: &ReleaseChannel) -> Option<&ReleaseChannelConfig> {
        self.channels
            .iter()
            .find(|config| config.channel == channel.channel)
    }
    fn show(&self) {
        println!("Default cluster version: {}", self.default_cluster_version);
        let mut channels = self.channels.iter().collect::<Vec<_>>();
        channels.sort_by_key(|config| config.channel.value());
        channels.iter().for_each(|config| {
            println!(
                "Channel {}: default version {}, upgrade target version {}",
                config.channel.name().unwrap_or_default(),
                config.default_version,
                config.upgrade_target_version
            )
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_server_config() -> ServerConfig {
        let channel_config = ReleaseChannelConfig::default()
            .set_channel(Channel::Regular)
            .set_upgrade_target_version("1.31.0")
            .set_valid_versions(["1.31.0", "1.30.0"]);

        ServerConfig::default()
            .set_default_cluster_version("1.30.0")
            .set_channels([channel_config])
    }

    #[test]
    fn release_channel_by_name_returns_matching_channel() {
        let config = make_server_config();
        let result = config.release_channel_by_name("REGULAR");
        assert!(result.is_some());
    }

    #[test]
    fn release_channel_by_name_returns_none_for_unknown() {
        let config = make_server_config();
        assert!(config.release_channel_by_name("RAPID").is_none());
    }

    #[test]
    fn release_channel_config_returns_matching_config() {
        let config = make_server_config();
        let channel = ReleaseChannel::new().set_channel(Channel::Regular);
        assert!(config.release_channel_config(&channel).is_some());
    }

    #[test]
    fn release_channel_config_returns_none_for_unknown_channel() {
        let config = make_server_config();
        let channel = ReleaseChannel::new().set_channel(Channel::Rapid);
        assert!(config.release_channel_config(&channel).is_none());
    }

    #[test]
    fn release_channel_upgrade_target_version_returns_version() {
        let config = make_server_config();
        let channel = ReleaseChannel::new().set_channel(Channel::Regular);
        assert_eq!(
            config.release_channel_upgrade_target_version(&channel),
            Some("1.31.0")
        );
    }

    #[test]
    fn cluster_without_release_channel_defaults_to_regular() {
        let cluster = Cluster::default();
        let channel = cluster.release_channel_or_regular();
        assert_eq!(channel.channel, Channel::Regular);
    }

    #[test]
    fn cluster_without_release_channel_name_is_none() {
        let cluster = Cluster::default();
        assert!(cluster.release_channel_name().is_none());
    }

    #[test]
    fn cluster_with_release_channel_returns_name() {
        let cluster = Cluster::default()
            .set_release_channel(ReleaseChannel::new().set_channel(Channel::Rapid));
        assert_eq!(cluster.release_channel_name(), Some("RAPID"));
    }
}
