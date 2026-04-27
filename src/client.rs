use super::*;

#[derive(Debug)]
pub enum UpgradeAction {
    Cluster,
    MasterOnly,
    AllNodePools,
    NodePools(Vec<String>),
}

impl UpgradeAction {
    pub fn from_args(master: bool, node_pools: Option<Vec<String>>) -> anyhow::Result<Self> {
        match (master, node_pools) {
            (true, Some(_)) => {
                anyhow::bail!("Cannot specify both master and node pools upgrade targets")
            }
            (true, None) => Ok(Self::MasterOnly),
            (false, None) => Ok(Self::Cluster),
            (false, Some(pools)) if pools.is_empty() => Ok(Self::AllNodePools),
            (false, Some(pools)) => Ok(Self::NodePools(pools)),
        }
    }
}

pub struct GkeClient {
    cm: gke::client::ClusterManager,
    location: String,
    project: String,
    config: gke::model::ServerConfig,
}

impl GkeClient {
    pub async fn new(location: Option<String>, project: Option<String>) -> anyhow::Result<Self> {
        let cm = gke::client::ClusterManager::builder().build().await?;
        let location = location.unwrap_or_else(|| "*".to_string());
        let project = project.unwrap_or_else(|| "*".to_string());
        let config = gke::model::ServerConfig::default();

        let client = Self {
            cm,
            location,
            project,
            config,
        };

        let config = client.get_server_config().await?;
        Ok(Self { config, ..client })
    }

    pub fn config(&self) -> &gke::model::ServerConfig {
        &self.config
    }

    pub async fn pull_up(
        &self,
        cluster: &str,
        action: UpgradeAction,
        target: Target,
    ) -> anyhow::Result<()> {
        let config = self.config();
        let cluster = self.get_cluster(cluster).await?;
        let channel = cluster.release_channel_or_regular();
        let version = target.find_compatible_version(config, channel)?;

        announce(&cluster);

        println!(
            "Pulling up cluster {} [{}] in location {} for project {} to version {} ({:?})",
            cluster.name, cluster.id, cluster.location, self.project, version, action
        );

        match action {
            UpgradeAction::Cluster => {
                self.upgrade_master(&cluster, version).await?;
                for pool in &cluster.node_pools {
                    self.upgrade_node_pool(&cluster, &pool.name, version)
                        .await?;
                }
            }
            UpgradeAction::MasterOnly => {
                self.upgrade_master(&cluster, version).await?;
            }
            UpgradeAction::AllNodePools => {
                for pool in &cluster.node_pools {
                    self.upgrade_node_pool(&cluster, &pool.name, version)
                        .await?;
                }
            }
            UpgradeAction::NodePools(pools) => {
                for pool in pools {
                    let operation = self.upgrade_node_pool(&cluster, &pool, version).await?;
                    self.track_operation(operation).await?;
                }
            }
        }
        Ok(())
    }

    async fn track_operation(
        &self,
        mut operation: gke::model::Operation,
    ) -> gke::Result<gke::model::Operation> {
        while operation.status != gke::model::operation::Status::Done {
            operation = self.get_operation(&operation).await?;
            println!(
                "{} [{}]: {}",
                operation.operation_type, operation.status, operation.detail
            );
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
        Ok(operation)
    }

    async fn upgrade_master(
        &self,
        cluster: &gke::model::Cluster,
        version: &str,
    ) -> gke::Result<gke::model::Operation> {
        println!(
            "Upgrading master in cluster {} to version {}",
            cluster.name, version
        );
        let name = format!(
            "projects/{}/locations/{}/clusters/{}",
            self.project, self.location, cluster.name
        );
        self.cm
            .update_master()
            .set_name(name)
            .set_master_version(version)
            .send()
            .await
            .inspect(|operation| println!("Operation: {operation:?}"))
    }

    async fn upgrade_node_pool(
        &self,
        cluster: &gke::model::Cluster,
        pool: &str,
        version: &str,
    ) -> gke::Result<gke::model::Operation> {
        println!(
            "Upgrading node pool {} in cluster {} to version {}",
            pool, cluster.name, version
        );
        let name = format!(
            "projects/{}/locations/{}/clusters/{}/nodePools/{}",
            self.project, self.location, cluster.name, pool
        );
        self.cm
            .update_node_pool()
            .set_name(name)
            .set_node_version(version)
            .send()
            .await
            .inspect(|operation| println!("Operation: {operation:?}"))
    }

    async fn get_cluster(&self, cluster: &str) -> gke::Result<gke::model::Cluster> {
        let name = format!(
            "projects/{}/locations/{}/clusters/{}",
            self.project, self.location, cluster
        );

        self.cm.get_cluster().set_name(name).send().await
    }

    pub async fn get_server_config(&self) -> gke::Result<gke::model::ServerConfig> {
        let name = format!("projects/{}/locations/{}", self.project, self.location);
        self.cm.get_server_config().set_name(name).send().await
    }

    async fn get_operation(
        &self,
        operation: &gke::model::Operation,
    ) -> gke::Result<gke::model::Operation> {
        let name = format!(
            "projects/{}/locations/{}/operations/{}",
            self.project, self.location, operation.name
        );
        self.cm.get_operation().set_name(name).send().await
    }
}

fn announce(cluster: &gke::model::Cluster) {
    let name = if cluster.description.is_empty() {
        cluster.name.clone()
    } else {
        format!("{} ({})", cluster.name, cluster.description)
    };

    let channel = cluster.release_channel_name().unwrap_or_default();

    println!("Cluster {name} (release channel: {channel})",);

    println!(
        "  Master endpoint [{}] is currently at version {}",
        cluster.status, cluster.current_master_version
    );
    for pool in &cluster.node_pools {
        println!(
            "  Node pool {} [{}] is currently at version {}",
            pool.name, pool.status, pool.version
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_args_no_flags_upgrades_full_cluster() {
        let action = UpgradeAction::from_args(false, None).unwrap();
        assert!(matches!(action, UpgradeAction::Cluster));
    }

    #[test]
    fn from_args_master_flag_upgrades_master_only() {
        let action = UpgradeAction::from_args(true, None).unwrap();
        assert!(matches!(action, UpgradeAction::MasterOnly));
    }

    #[test]
    fn from_args_master_and_node_pools_is_error() {
        let result = UpgradeAction::from_args(true, Some(vec!["pool-1".to_string()]));
        assert!(result.is_err());
    }

    #[test]
    fn from_args_empty_node_pools_upgrades_all_pools() {
        let action = UpgradeAction::from_args(false, Some(vec![])).unwrap();
        assert!(matches!(action, UpgradeAction::AllNodePools));
    }

    #[test]
    fn from_args_named_node_pools_upgrades_those_pools() {
        let pools = vec!["pool-1".to_string(), "pool-2".to_string()];
        let action = UpgradeAction::from_args(false, Some(pools.clone())).unwrap();
        assert!(matches!(action, UpgradeAction::NodePools(p) if p == pools));
    }
}
