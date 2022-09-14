# Pyrsia

Microservice Configuration Management - Track, Version, Find, Share and Deploy Microservices

[Overview of Pyrsia](https://pyrsia.io)

## TL;DR - Google

```console
gcloud projects add-iam-policy-binding <PROJECT> --member=serviceAccount:service-<PROJECT_NUMBER>@compute-system.iam.gserviceaccount.com --role=roles/cloudkms.cryptoKeyEncrypterDecrypter

kubectl create namespace pyrsia
helm repo add pyrsiaoss https://helmrepo.pyrsia.io/repos/nightly
helm repo update
helm upgrade --install pyrsia -n pyrsia pyrsiaoss/pyrsia-node --set "k8s_provider=gke" --set "p2pkeys.kms_key_id=projects/<PROJECT>/locations/global/keyRings/<KEYRING>/cryptoKeys/<KEY>"
```

## TL;DR - Oracle

```console
oci .......

kubectl create namespace pyrsia
helm repo add pyrsiaoss https://helmrepo.pyrsia.io/repos/nightly
helm repo update
helm upgrade --install pyrsia -n pyrsia pyrsiaoss/pyrsia-node --set "k8s_provider=oke" --set "p2pkeys.kms_key_id=<KMS KEY>"
```

## Introduction

This chart deploys all of the required secrets, services, and deployments on a [Kubernetes](https://kubernetes.io) cluster using the [Helm](https://helm.sh) package manager.

## Prerequisites

- Kubernetes 1.19+
- Helm 3.2.0+
- KMS Key to Encrypt Pyrsia Keys Volume

## Installing the Chart

To install the chart with the release name `pyrsia`:

```console
kubectl create namespace pyrsia
helm repo add pyrsiaoss https://helmrepo.pyrsia.io/repos/nightly
helm repo update
helm upgrade --install pyrsia -n pyrsia pyrsiaoss/pyrsia-node --set "k8s_provider=<gke,oke,aks,eks>" --set "p2pkeys.kms_key_id=<KEY>"
```

The command deploys Pyrsia on the Kubernetes cluster using the following parameters:

- p2pkeys.kms_key_id = KMS Key to Encrypt Pyrsia Keys Volume
  - Google
    1. Enable Cloud KMS API
    2. Assign the Cloud KMS CryptoKey Encrypter/Decrypter role (roles/cloudkms.cryptoKeyEncrypterDecrypter) to the Compute Engine Service Agent (service-[PROJECT_NUMBER]@compute-system.iam.gserviceaccount.com)

       ```console
       gcloud projects add-iam-policy-binding <PROJECT> --member=serviceAccount:service-<PROJECT_NUMBER>@compute-system.iam.gserviceaccount.com --role=roles/cloudkms.cryptoKeyEncrypterDecrypter
       ```

  - Oracle (to be filled out in next PR by Oracle)
    1. Placeholder for Step One
    2. Placeholder for Step Two

> **Tip**: List all releases using `helm list`

## Uninstalling the Chart

To uninstall/delete the `pyrsia` deployment:

```console
helm delete pyrsia -n pyrsia
```

The command removes all the Kubernetes components associated with the chart and deletes the release.

## Parameters

### Common parameters

| Name                     | Description                                   | Value           |
| ------------------------ | ----------------------------------------------| --------------- |
| `k8s_provider`           | Environment that is running Kubernetes        | gke, aks, eks, oke |
| `p2pkeys.kms_key_id`     | KMS Key to Encrypt Pyrsia Keys Volume         | Name of the CSI Key.  For example, under GKE: `projects/<PROJECT>locations/global/keyRings/<KEYRING>/cryptoKeys/<KEY>` |

> NOTE: Once this chart is deployed, it is not possible to change the application's access credentials, such as usernames or passwords, using Helm. To change these application credentials after deployment, delete any persistent volumes (PVs) used by the chart and re-deploy it, or use the application's built-in administrative tools if available.

Alternatively, a YAML file that specifies the values for the above parameters can be provided while installing the chart. For example,

```console
helm upgrade --install pyrsia -n pyrsia -f values.yaml pyrsiaoss/pyrsia-node
```
