# Pyrsia

Microservice Configuration Management - Track, Version, Find, Share and Deploy Microservices

[Overview of Pyrsia](https://pyrsia.io)

## TL;DR - Google

```console
#!/usr/bin/env bash

# GKE Project Details
export GKE_PROJECT_ID=$(gcloud config get project)
export GKE_PROJECT_NUMBER=$(gcloud projects list --filter="${GKE_PROJECT_ID}" --format="value(PROJECT_NUMBER)")

# External DNS
export DNS_SA_EMAIL="${DNS_SA_NAME}@${GKE_PROJECT_ID}.iam.gserviceaccount.com"
export DNS_SA_NAME="external-dns-sa"
export EXTERNALDNS_NS="external-dns"
export EXTERNALDNS_DOMAIN="example.com"

# Pyrsia P2P
export KEY="pyrsia-p2p-key"
export KEYRING="pyrsia-keyring"
export PYRSIA_NS="pyrsia"

# Assign google service account to cloudkms.cryptoKeyEncrypterDecrypter role in project
gcloud projects add-iam-policy-binding ${GKE_PROJECT_ID} --member=serviceAccount:service-${GKE_PROJECT_NUMBER}@compute-system.iam.gserviceaccount.com --role=roles/cloudkms.cryptoKeyEncrypterDecrypter

# Create GSA used to access the Cloud DNS zone
gcloud iam service-accounts create ${DNS_SA_NAME} --display-name ${DNS_SA_NAME}

# Assign google service account to dns.admin role in project
gcloud projects add-iam-policy-binding ${GKE_PROJECT_ID} --member serviceAccount:${DNS_SA_EMAIL} --role "roles/dns.admin"

# Download static credentials for ExternalDNS
gcloud iam service-accounts keys create credentials.json --iam-account ${DNS_SA_EMAIL}

# Save the credentials as a Secret for ExternalDNS
kubectl create namespace ${EXTERNALDNS_NS}
kubectl create secret generic "external-dns" --namespace ${EXTERNALDNS_NS} --from-file credentials.json

# Create Keyring and Key for Pyrsia P2P
gcloud kms keyrings create ${KEYRING} --project ${GKE_PROJECT_ID} --location global
gcloud kms keys create ${KEY} --keyring ${KEYRING} --project ${GKE_PROJECT_ID} --location global --purpose "Symmetric encrypt/decrypt"

# Install Pyrsia
kubectl create namespace ${PYRSIA_NS}
helm repo add pyrsiaoss https://helmrepo.pyrsia.io/repos/nightly
helm repo update
helm upgrade pyrsia pyrsiaoss/pyrsia-node -n ${PYRSIA_NS} --install --set "k8s_provider=gke" --set "p2pkeys.kms_key_id=projects/${GKE_PROJECT_ID}/locations/global/keyRings/${KEYRING}/cryptoKeys/${KEY}" --set external_dns_ns=${EXTERNALDNS_NS} --set dnsname=${EXTERNALDNS_DOMAIN}

```

## TL;DR - Oracle (WIP)

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
| `external_dns_ns`        | Namespace for the External DNS pod/service | default: external-dns |
| `dnsname`                | DNS Name for the nodes                     | default: pyrsia.link  |
| `buildnode`              | URL for the Build Node                     | |

> NOTE: Once this chart is deployed, it is not possible to change the application's access credentials, such as usernames or passwords, using Helm. To change these application credentials after deployment, delete any persistent volumes (PVs) used by the chart and re-deploy it, or use the application's built-in administrative tools if available.

Alternatively, a YAML file that specifies the values for the above parameters can be provided while installing the chart. For example,

```console
helm upgrade --install pyrsia -n pyrsia -f values.yaml pyrsiaoss/pyrsia-node
```
