# Managing Pyrsia on Kubernetes

## Prerequisites

### GKE - Google

- [Install gcloud](https://cloud.google.com/sdk/docs/install-sdk)
- Set your gcloud config

    ```toml
    [compute]
    zone = us-central1-c
    [container]
    cluster = pyrsia-cluster-1
    [core]
    disable_usage_reporting = False
    project = pyrsia-sandbox
    ```

- [Install kubectl](https://cloud.google.com/kubernetes-engine/docs/how-to/cluster-access-for-kubectl#install_kubectl)
- [Intall GKE Auth Plugin](https://cloud.google.com/kubernetes-engine/docs/how-to/cluster-access-for-kubectl#install_plugin)
- [Configure Access to Cluster](https://cloud.google.com/sdk/gcloud/reference/container/clusters/get-credentials) - pyrsia-nighty (has spelling mistake) or pyrsia-cluster-1

    ```bash
    gcloud container clusters get-credentials <CLUSTER_NAME>
    ```

- [Install Helm](https://helm.sh/docs/intro/install/)

### EKS - Amazon

- [Install aws-cli](https://docs.aws.amazon.com/cli/latest/userguide/getting-started-install.html#getting-started-install-instructions)
- [Install kubectl](https://kubernetes.io/docs/tasks/tools/)
- [Configure Access to Cluster](https://docs.aws.amazon.com/cli/latest/reference/eks/update-kubeconfig.html)

    ```bash
    aws eks update-kubeconfig --name <CLUSTER_NAME>
    ```

- [Install Helm](https://helm.sh/docs/intro/install/)

## Interacting with the clusters

| Action | Command |
| ----   | ------- |
| List current cluster connection |`kubectl config view --minify -o jsonpath='{.clusters[].name}'` |
| List current contexts | `kubectl config get-contexts` |
| Switch to another cluster | `kubectl config use-context <context name>` |
| List running pods | `kubectl get pods -n pyrsia-node` |
| List public ip assigned to pods | `kubectl get svc -n pyrsia-node` |
| Get logs for pod | `kubectl logs -n pyrsia-node pyrsia-node-0` |
| "ssh" to pod | `kubectl exec -it -n pyrsia-node pyrsia-node-0 -- bash` |
| "reboot" a pod | `kubectl delete pod -n pyrsia-node pyrsia-node-0` |
| Get image tag for a pod | `kubectl describe pod -n pyrsia-node pyrsia-node-0` |
| Get ingress details | `kubectl describe svc -n pyrsia-node pyrsia-node-0` |

## Using helm to deploy updates

The helm charts are automatically published by the Github Actions to the <http://helmrepo.pyrsia.io> site. [ArtifactHub](https://artifacthub.io) pulls in chart updates every half hour.

Details about the Chart Values and Installation steps are documented in the chart's ReadMe.  ArtifactHub renders the ReadMe on the [Prysia Package](https://artifacthub.io/packages/helm/pyrsia-nightly/pyrsia-node) site.

1. Add the remote repo location to helm
`helm repo add pyrsia-nightly http://helmrepo.pyrsia.io`
2. Fetch the latest charts
`helm repo update`
3. Set you cluster connection
`kubectl config use-context <context name>`
4. Deployment
    - Nightly

        ```bash
        helm upgrade --install node1 -n pyrsia-node pyrsia-nightly/pyrsia-node --set "k8s_provider=gke" --set "p2pkeys.kms_key_id=projects/pyrsia-sandbox/locations/global/keyRings/pyrsia-keyring/cryptoKeys/pyrsia-p2p-key" --set "dnsname=nightly.pyrsia.link" --set "bootdns=nightly.pyrsia.link"  --set "replicaCount=1" --set "buildnode=http://35.193.148.20:8080" --version "0.2.1-2562"
        ```

    - Production

        ```bash
        helm upgrade --install node1 -n pyrsia-node pyrsia-nightly/pyrsia-node --set "k8s_provider=gke" --set "p2pkeys.kms_key_id=projects/pyrsia-sandbox/locations/global/keyRings/pyrsia-keyring/cryptoKeys/pyrsia-p2p-key"   --set "replicaCount=1"  --set "buildnode=http://34.134.11.239:8080" --version "0.2.1-2562"
        ```

    > Note: Change the --version of the chart to reflect the image tag you want to deploy.  The image tag and chart version are kept in sync.

Verify the deployments using `kubectl` commands.

## Other helm commands

- List Deployments

    `helm list`

- Delete Deployment

    `helm delete -n pyrsia-node node1`

## Cheatsheets

- [kubectl](https://kubernetes.io/docs/reference/kubectl/cheatsheet/#viewing-finding-resources)
- [helm](https://phoenixnap.com/kb/helm-commands-cheat-sheet)
- [gcloud](https://cloud.google.com/sdk/docs/cheatsheet)
