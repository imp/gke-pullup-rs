#![warn(clippy::cast_possible_truncation)]
#![warn(clippy::cloned_instead_of_copied)]
#![warn(clippy::flat_map_option)]
#![warn(clippy::implicit_clone)]
#![warn(clippy::map_flatten)]
#![warn(clippy::map_unwrap_or)]
#![warn(clippy::unused_trait_names)]
#![warn(clippy::unused_async)]
#![warn(clippy::use_self)]
// #![warn(clippy::large_futures)]
#![warn(deprecated_in_future)]
#![warn(future_incompatible)]
#![warn(noop_method_call)]
#![warn(missing_debug_implementations)]
#![warn(rust_2018_compatibility)]
#![warn(rust_2021_compatibility)]
#![warn(rust_2024_compatibility)]
#![warn(rust_2018_idioms)]
#![warn(unused)]
#![deny(warnings)]

use clap::Parser;
use clap::Subcommand;
use google_cloud_container_v1 as gke;
// use serde_json as json;

use client::UpgradeAction;
use ext::ClusterExt as _;
use ext::ServerConfigExt as _;
use target::Target;

mod client;
mod ext;
mod target;

#[derive(Debug, Parser)]
struct Cli {
    #[clap(long, global = true, hide = true)]
    debug: bool,
    #[clap(long, global = true)]
    location: Option<String>,
    #[clap(long, global = true)]
    project: Option<String>,
    #[clap(subcommand)]
    command: Command,
}

impl Cli {
    fn inspect(self) -> Self {
        if self.debug {
            println!("{self:?}");
        }
        self
    }

    async fn exec(self) -> anyhow::Result<()> {
        let project = self.project.or_else(load_default_project);
        let gke = client::GkeClient::new(self.location, project).await?;
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
    /// Print the server config for the current project and location.
    ServerConfig,
}

impl Command {
    async fn exec(self, gke: &client::GkeClient) -> anyhow::Result<()> {
        match self {
            Self::PullUp {
                cluster,
                target,
                master,
                node_pools,
            } => {
                let upgrade_action = UpgradeAction::from_args(master, node_pools)?;
                let config = gke.config();
                let target = target::Target::new(target, config)
                    .inspect(|target| println!("cli target: {target:?}"))?;
                gke.pull_up(&cluster, upgrade_action, target).await
            }
            Self::ServerConfig => {
                gke.config().show();
                Ok(())
            }
        }
    }
}

fn load_default_project() -> Option<String> {
    let stdout = std::process::Command::new("gcloud")
        .args(["config", "get", "project"])
        .output()
        .ok()?
        .stdout;
    let project = String::from_utf8_lossy(&stdout).trim().to_string();
    if project.is_empty() {
        None
    } else {
        Some(project)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    Cli::parse().inspect().exec().await
}
