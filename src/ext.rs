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
}
