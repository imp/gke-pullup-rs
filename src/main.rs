use clap::Parser;
use clap::Subcommand;
use google_cloud_container_v1 as gke;

use client::UpgradeAction;
use ext::ClusterExt;
use ext::ServerConfigExt;
use target::Target;

mod client;
mod ext;
mod target;

#[derive(Debug, Parser)]
struct Cli {
    #[clap(long, global = true)]
    location: Option<String>,
    #[clap(long, global = true)]
    project: Option<String>,
    #[clap(subcommand)]
    command: Command,
}

impl Cli {
    fn inspect(self) -> Self {
        println!("{self:?}");
        self
    }

    async fn exec(self) -> anyhow::Result<()> {
        let gke = client::GkeClient::new(self.location, self.project).await?;
        self.command.exec(&gke).await
    }
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Pull up a GKE cluster to the latest version compatible with its release channel.
    PullUp {
        /// The name of the cluster to pull up. By default it will upgrade both master and node pools.
        cluster: String,
        /// Only upgrade the master version, not the node pools.
        #[clap(long)]
        master: bool,
        /// Only upgrade the node pools, not the master version.
        /// The node pools will be upgraded to the master version.
        /// If no values are provided, all the node pools will be upgraded.
        /// If specific node pools are provided, only those node pools will be upgraded.
        #[clap(long, num_args = 0.., value_delimiter = ',')]
        node_pools: Option<Vec<String>>,
        /// Upgrade to a specific version instead of the latest compatible version.
        /// The version can be either one of the versions compatible with the cluster's release channel
        /// or the name of the release channel itself, in which case the cluster will be upgraded to the
        /// latest compatible version for that channel.
        target: Option<String>,
    },
}

impl Command {
    async fn exec(self, gke: &client::GkeClient) -> anyhow::Result<()> {
        match self {
            Command::PullUp {
                cluster,
                target,
                master,
                node_pools,
            } => {
                let upgrade_action = UpgradeAction::from_args(master, node_pools)?;
                let config = gke.get_server_config().await?;
                println!(
                    "Default cluster version: {}",
                    config.default_cluster_version
                );

                config.channels.iter().for_each(|config| {
                    println!(
                        "Channel {}: default version {}, upgrade target version {}",
                        config.channel.name().unwrap_or_default(),
                        config.default_version,
                        config.upgrade_target_version
                    )
                });
                let target = target::Target::new(target, config)
                    .inspect(|target| println!("cli target: {}", target.short()))?;
                gke.pull_up(&cluster, upgrade_action, target).await
            }
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    Cli::parse().inspect().exec().await
}
