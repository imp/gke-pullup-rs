# gke-pullup

A command-line tool to upgrade GKE (Google Kubernetes Engine) clusters to the
latest Kubernetes version compatible with their release channel.

## Features

- Upgrades both the cluster master and all node pools in one command
- Supports upgrading only the master, only specific node pools, or all node pools
- Respects the cluster's configured release channel (Rapid, Regular, Stable)
- Allows targeting a specific Kubernetes version or a specific release channel
- Automatically fetches the GKE server config to validate versions before upgrading

## Prerequisites

- A Google Cloud project with at least one GKE cluster
- Application Default Credentials configured (`gcloud auth application-default login`)

## Installation

```bash
cargo install gke-pullup
```

## Usage

```
gke-pullup [OPTIONS] <COMMAND>
```

### Global options

| Option | Description |
|---|---|
| `--location <LOCATION>` | GCP location (region or zone) of the cluster. Defaults to `*` (all locations). |
| `--project <PROJECT>` | GCP project ID. Defaults to `*` (all projects). |

### Commands

#### `pull-up`

Upgrade a cluster to the latest version compatible with its release channel.

```
gke-pullup [--location <LOCATION>] [--project <PROJECT>] pull-up [OPTIONS] <CLUSTER> [TARGET]
```

| Argument / Option | Description |
|---|---|
| `<CLUSTER>` | Name of the GKE cluster to upgrade. |
| `[TARGET]` | Optional target: a Kubernetes version string (e.g. `1.31.2-gke.100`) or a release channel name (`RAPID`, `REGULAR`, `STABLE`). Defaults to the upgrade target version for the cluster's own release channel. |
| `--master` | Upgrade only the master control plane. |
| `--node-pools [POOL,...]` | Upgrade node pools only. Without values, upgrades all node pools. With a comma-separated list, upgrades only the named pools. |

`--master` and `--node-pools` are mutually exclusive. When neither is provided, both the master and all node pools are upgraded.

### Examples

Upgrade the cluster `my-cluster` (master + all node pools) to the latest version for its release channel:

```bash
gke-pullup --project my-project --location us-central1 pull-up my-cluster
```

Upgrade only the master of `my-cluster`:

```bash
gke-pullup --project my-project --location us-central1 pull-up --master my-cluster
```

Upgrade all node pools of `my-cluster` to the current master version:

```bash
gke-pullup --project my-project --location us-central1 pull-up --node-pools my-cluster
```

Upgrade specific node pools:

```bash
gke-pullup --project my-project --location us-central1 pull-up --node-pools pool-1,pool-2 my-cluster
```

Upgrade to a specific Kubernetes version:

```bash
gke-pullup --project my-project --location us-central1 pull-up my-cluster 1.31.2-gke.100
```

Upgrade to the latest version of the `RAPID` channel:

```bash
gke-pullup --project my-project --location us-central1 pull-up my-cluster RAPID
```

## License

Apache-2.0
