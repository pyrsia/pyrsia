# Pyrsia

Microservice Configuration Management - Track, Version, Find, Share and Deploy Microservices

[Overview of Pyrsia](https://pyrsia.io)

## TL;DR

```console
gcloud projects add-iam-policy-binding <PROJECT> --member=serviceAccount:service-<PROJECT_NUMBER>@compute-system.iam.gserviceaccount.com --role=roles/cloudkms.cryptoKeyEncrypterDecrypter

helm repo add pyrsiaoss https://helmrepo.pyrsia.io/
helm install my-release pyrsiaoss/pyrsia-node --set "p2pkeys.kms_key_id=projects/<PROJECT>locations/global/keyRings/<KEYRING>/cryptoKeys/<KEY>"
```

## Introduction

This chart deploys all of the required secrets, services, and deployments on a [Kubernetes](https://kubernetes.io) cluster using the [Helm](https://helm.sh) package manager.

## Prerequisites

- Kubernetes 1.19+
- Helm 3.2.0+
- KMS Key to Encrypt Pyrsia Keys Volume

## Installing the Chart

To install the chart with the release name `my-release`:

```console
helm repo add pyrsiaoss https://helmrepo.pyrsia.io/
helm install my-release pyrsiaoss/pyrsia-node --set "p2pkeys.kms_key_id=projects/<PROJECT>locations/global/keyRings/<KEYRING>/cryptoKeys/<KEY>"
```

The command deploys Pyrsia on the Kubernetes cluster using the following parameters:

- p2pkeys.kms_key_id = KMS Key to Encrypt Pyrsia Keys Volume
  - Google
    1. Enable Cloud KMS API
    2. Assign the Cloud KMS CryptoKey Encrypter/Decrypter role (roles/cloudkms.cryptoKeyEncrypterDecrypter) to the Compute Engine Service Agent (service-[PROJECT_NUMBER]@compute-system.iam.gserviceaccount.com)

       ```console
       gcloud projects add-iam-policy-binding <PROJECT> --member=serviceAccount:service-<PROJECT_NUMBER>@compute-system.iam.gserviceaccount.com --role=roles/cloudkms.cryptoKeyEncrypterDecrypter
       ```

> **Tip**: List all releases using `helm list`

## Uninstalling the Chart

To uninstall/delete the `my-release` deployment:

```console
helm delete my-release
```

The command removes all the Kubernetes components associated with the chart and deletes the release.

## Parameters

### Common parameters

| Name                     | Description                                                                                  | Value           |
| ------------------------ | -------------------------------------------------------------------------------------------- | --------------- |
| `p2pkeys.kms_key_id`     | KMS Key to Encrypt Pyrsia Keys VolumeName                                                                  | projects/<PROJECT>locations/global/keyRings/<KEYRING>/cryptoKeys/<KEY> |

> NOTE: Once this chart is deployed, it is not possible to change the application's access credentials, such as usernames or passwords, using Helm. To change these application credentials after deployment, delete any persistent volumes (PVs) used by the chart and re-deploy it, or use the application's built-in administrative tools if available.

Alternatively, a YAML file that specifies the values for the above parameters can be provided while installing the chart. For example,

```console
helm install my-release -f values.yaml pyrsiaoss/pyrsia-node
```
