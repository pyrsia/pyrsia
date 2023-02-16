# Pyrsia

Microservice Configuration Management - Track, Version, Find, Share and Deploy Microservices

[Overview of Pyrsia](https://pyrsia.io)

Setup and Installation Instructions

- [Google - GKE](https://pyrsia.io/docs/SRE/gke-setup)
- [AWS - EKS](https://pyrsia.io/docs/SRE/eks-setup)
- Oracle - OCI: Work in progress

## Introduction

This chart deploys all of the required secrets, services, and deployments on a [Kubernetes](https://kubernetes.io) cluster using the [Helm](https://helm.sh) package manager.

## Prerequisites

- Kubernetes 1.19+
- Helm 3.2.0+

## Installing the Chart

To install the chart with the release name `node1`:

```console
kubectl create namespace pyrsia-node
helm repo add pyrsiaoss https://helmrepo.pyrsia.io/repos/nightly
helm repo update
helm upgrade --install node1 -n pyrsia-node pyrsiaoss/pyrsia-node --version "0.2.4+3003"
```

[Further details](https://pyrsia.io/docs/SRE/kubernetes-helm) on deploying to multiple cloud providers

> **Tip**: List all releases using `helm list`

## Uninstalling the Chart

To uninstall/delete the `pyrsia` deployment:

```console
helm delete pyrsia -n pyrsia
```

The command removes all the Kubernetes components associated with the chart and deletes the release.

## Parameters

### Common parameters

| Name                    | Description                                | Value           |
| ----------------------- | -------------------------------------------| --------------- |
| `domain`                | DNS Name for the nodes                     | default: pyrsia.link  |
| `bootdns`               | URL for the Build Node                     | default: boot.pyrsia.link |
| `buildnode`             | URL for the Build Node                     | |
| `keys.p2p`              | ed25519.ser for the libp2p                 | default will auto generate a key pair |
| `keys.blockchain`       | ed25519.ser for the libp2p                 | default will auto generate a key pair |

> NOTE: Once this chart is deployed, it is not possible to change the application's access credentials, such as usernames or passwords, using Helm. To change these application credentials after deployment, delete any persistent volumes (PVs) used by the chart and re-deploy it, or use the application's built-in administrative tools if available.

Alternatively, a YAML file that specifies the values for the above parameters can be provided while installing the chart. For example,

```console
helm upgrade --install pyrsia -n pyrsia -f values.yaml pyrsiaoss/pyrsia-node
```
