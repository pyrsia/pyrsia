# Managing Pyrsia on Kubernetes

## Prerequisites

### GKE - Google

- [Install gcloud](https://cloud.google.com/sdk/docs/install-sdk)
- Set your gcloud config (Refer to [gcloud documentation](https://cloud.google.com/sdk/gcloud/reference/config/set) for how-to)

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
    gcloud container clusters get-credentials <CLUSTER_NAME> -z <ZONE>
    # e.g.
    gcloud container clusters get-credentials pyrsia-nighty -z us-central1-c
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

## Using helm to deploy for first time to a cluster

The helm charts are automatically published by the Github Actions to the <http://helmrepo.pyrsia.io> site. [ArtifactHub](https://artifacthub.io) pulls in chart updates every half hour.

Details about the Chart Values and Installation steps are documented in the chart's ReadMe.  ArtifactHub renders the ReadMe on the [Prysia Package](https://artifacthub.io/packages/helm/pyrsia-nightly/pyrsia-node) site.

1. Add the remote repo location to helm
`helm repo add pyrsia-nightly https://helmrepo.pyrsia.io/repos/nightly/`

2. Fetch the latest charts
`helm repo update`

3. Set you cluster connection
`kubectl config use-context <context name>`

4. Obtain the Key Pairs from Last Pass
    - staging_gke_ed25519.pem
    - staging_eks_ed25519.pem
    - prod_gke_ed25519.pem
    - prod_eks_ed25519.pem

5. Deployment

    - Staging for GKE
        - Setup Environment Variables

            ```bash
            CHART_VERSION=0.2.4+3003
            BUILD_CHART_VERSION=0.1.0+7
            CLUSTER_CONFIG=gke_pyrsia-sandbox_us-central1_pyrsia-staging
            kubectl config use-context ${CLUSTER_CONFIG}
            PYRSIA_NAMESPACE=pyrsia-node
            PYRSIA_BASE_DOMAIN=pyrsia.link
            PYRSIA_DOMAIN=staging.${PYRSIA_BASE_DOMAIN}
            PYRSIA_BOOTDNS=boot.${PYRSIA_DOMAIN}
            PYRSIA_KEYPAIR=staging_gke_ed25519
            ```

        - Deploy

            ```bash
            PATH=/usr/local/Cellar/openssl@3/3.0.7/bin:$PATH
            openssl pkey -in ${PYRSIA_KEYPAIR}.pem -out - -outform DER | tail -c +17 > ${PYRSIA_KEYPAIR}.ser
            openssl pkey -in ${PYRSIA_KEYPAIR}.pem -pubout -outform DER | tail -c +13 >> ${PYRSIA_KEYPAIR}.ser

            helm upgrade --install node1 -n "${PYRSIA_NAMESPACE}" pyrsia-nightly/pyrsia-node --set "domain=${PYRSIA_DOMAIN}" --set "bootdns=${PYRSIA_BOOTDNS}" --set keys.p2p=$(cat ${PYRSIA_KEYPAIR}.ser | base64) --set keys.blockchain=$(cat ${PYRSIA_KEYPAIR}.ser | base64) --version "${CHART_VERSION}"
            helm upgrade build1 --install -n pyrsia-node pyrsia-nightly/pyrsia-build-service --set "bootdns=${PYRSIA_BOOTDNS}" --version "${BUILD_CHART_VERSION}"
            ```

    - Staging for EKS
        - Setup Environment Variables

            ```bash
            CHART_VERSION=0.2.4+3003
            BUILD_CHART_VERSION=0.1.0+7
            CLUSTER_CONFIG=sbtaylor@pyrsia-staging.us-east-1.eksctl.io
            kubectl config use-context ${CLUSTER_CONFIG}
            PYRSIA_NAMESPACE=pyrsia-node
            PYRSIA_BASE_DOMAIN=pyrsia-aws.link
            PYRSIA_DOMAIN=staging.${PYRSIA_BASE_DOMAIN}
            PYRSIA_BOOTDNS=boot.${PYRSIA_DOMAIN}
            PYRSIA_KEYPAIR=staging_eks_ed25519.ser
            ```

        - Deploy

            ```bash
            PATH=/usr/local/Cellar/openssl@3/3.0.7/bin:$PATH
            openssl pkey -in ${PYRSIA_KEYPAIR}.pem -out - -outform DER | tail -c +17 > ${PYRSIA_KEYPAIR}.ser
            openssl pkey -in ${PYRSIA_KEYPAIR}.pem -pubout -outform DER | tail -c +13 >> ${PYRSIA_KEYPAIR}.ser

            helm upgrade --install node1 -n "${PYRSIA_NAMESPACE}" pyrsia-nightly/pyrsia-node  --set "domain=${PYRSIA_DOMAIN}" --set "bootdns=${PYRSIA_BOOTDNS}" --set keys.p2p=$(cat ${PYRSIA_KEYPAIR}.ser | base64) --set keys.blockchain=$(cat ${PYRSIA_KEYPAIR}.ser | base64) --version "${CHART_VERSION}"
            helm upgrade build1 --install -n pyrsia-node pyrsia-nightly/pyrsia-build-service --set "bootdns=${PYRSIA_BOOTDNS}" --version "${BUILD_CHART_VERSION}"
            ```

    - Staging for GKE from branch

        - Setup Environment Variables

            ```bash
            CHART_VERSION=0.2.4+3003
            BUILD_CHART_VERSION=0.1.0+7
            CLUSTER_CONFIG=gke_pyrsia-sandbox_us-central1_pyrsia-staging
            kubectl config use-context ${CLUSTER_CONFIG}
            PYRSIA_NAMESPACE=pyrsia-node
            PYRSIA_BASE_DOMAIN=pyrsia.link
            PYRSIA_DOMAIN=staging.${PYRSIA_BASE_DOMAIN}
            PYRSIA_BOOTDNS=boot.${PYRSIA_DOMAIN}
            PYRSIA_KEYPAIR=staging_gke_ed25519
            IMAGE_REPO=mydockerrepo/pyrsia
            IMAGE_TAG=1.0
            ```

        - Deploy
            From the root of your Pyrsia repo:

            ```bash
            docker login
            docker build --tag ${IMAGE_REPO}:${IMAGE_TAG}
            docker push ${IMAGE_REPO}:${IMAGE_TAG}

            PATH=/usr/local/Cellar/openssl@3/3.0.7/bin:$PATH
            openssl pkey -in ${PYRSIA_KEYPAIR}.pem -out - -outform DER | tail -c +17 > ${PYRSIA_KEYPAIR}.ser
            openssl pkey -in ${PYRSIA_KEYPAIR}.pem -pubout -outform DER | tail -c +13 >> ${PYRSIA_KEYPAIR}.ser

            helm upgrade --install node1 -n "${PYRSIA_NAMESPACE}" pyrsia-nightly/pyrsia-node --set "domain=${PYRSIA_DOMAIN}" --set "bootdns=${PYRSIA_BOOTDNS}" --set keys.p2p=$(cat ${PYRSIA_KEYPAIR}.ser | base64) --set keys.blockchain=$(cat ${PYRSIA_KEYPAIR}.ser | base64) --version "${CHART_VERSION}" --set "image.repository=${IMAGE_REPO}" --set "image.tag=${IMAGE_TAG}"
            helm upgrade build1 --install -n pyrsia-node pyrsia-nightly/pyrsia-build-service --set "bootdns=${PYRSIA_BOOTDNS}" --version "${BUILD_CHART_VERSION}"
            ```

    - Staging for EKS from branch

        - Setup Environment Variables

            ```bash
            CHART_VERSION=0.2.4+3003
            BUILD_CHART_VERSION=0.1.0+7
            CLUSTER_CONFIG=sbtaylor@pyrsia-staging.us-east-1.eksctl.io
            kubectl config use-context ${CLUSTER_CONFIG}
            PYRSIA_NAMESPACE=pyrsia-node
            PYRSIA_BASE_DOMAIN=pyrsia.link
            PYRSIA_DOMAIN=staging.${PYRSIA_BASE_DOMAIN}
            PYRSIA_BOOTDNS=boot.${PYRSIA_DOMAIN}
            PYRSIA_KEYPAIR=staging_eks_ed25519
            IMAGE_REPO=mydockerrepo/pyrsia
            IMAGE_TAG=1.0
            ```

        - Deploy

            From the root of your Pyrsia repo:

            ```bash
            docker login
            docker build --tag ${IMAGE_REPO}:${IMAGE_TAG}
            docker push ${IMAGE_REPO}:${IMAGE_TAG}

            PATH=/usr/local/Cellar/openssl@3/3.0.7/bin:$PATH
            openssl pkey -in ${PYRSIA_KEYPAIR}.pem -out - -outform DER | tail -c +17 > ${PYRSIA_KEYPAIR}.ser
            openssl pkey -in ${PYRSIA_KEYPAIR}.pem -pubout -outform DER | tail -c +13 >> ${PYRSIA_KEYPAIR}.ser

            helm upgrade --install node1 -n "${PYRSIA_NAMESPACE}" pyrsia-nightly/pyrsia-node --set "domain=${PYRSIA_DOMAIN}" --set "bootdns=${PYRSIA_BOOTDNS}" --set keys.p2p=$(cat ${PYRSIA_KEYPAIR}.ser | base64) --set keys.blockchain=$(cat ${PYRSIA_KEYPAIR}.ser | base64) --version "${CHART_VERSION}" --set "image.repository=${IMAGE_REPO}" --set "image.tag=${IMAGE_TAG}"
            helm upgrade build1 --install -n pyrsia-node pyrsia-nightly/pyrsia-build-service --set "bootdns=${PYRSIA_BOOTDNS}" --version "${BUILD_CHART_VERSION}"
            ```

    - Production for GKE

        - Setup Environment Variables

            ```bash
            CHART_VERSION=0.2.4+3003
            BUILD_CHART_VERSION=0.1.0+7
            CLUSTER_CONFIG=gke_pyrsia-sandbox_us-central1-c_pyrsia-cluster-1
            kubectl config use-context ${CLUSTER_CONFIG}
            PYRSIA_NAMESPACE=pyrsia-node
            PYRSIA_KEYPAIR=prod_gke_ed25519
            ```

        - Deploy

            ```bash
            PATH=/usr/local/Cellar/openssl@3/3.0.7/bin:$PATH
            openssl pkey -in ${PYRSIA_KEYPAIR}.pem -out - -outform DER | tail -c +17 > ${PYRSIA_KEYPAIR}.ser
            openssl pkey -in ${PYRSIA_KEYPAIR}.pem -pubout -outform DER | tail -c +13 >> ${PYRSIA_KEYPAIR}.ser

            helm upgrade --install node1 -n ${PYRSIA_NAMESPACE} pyrsia-nightly/pyrsia-node --set keys.p2p=$(cat ${PYRSIA_KEYPAIR}.ser | base64) --set keys.blockchain=$(cat ${PYRSIA_KEYPAIR}.ser | base64) --version "${CHART_VERSION}"
            helm upgrade build1 --install -n pyrsia-node pyrsia-nightly/pyrsia-build-service --version "${BUILD_CHART_VERSION}"
            ```

    - Production for EKS

        - Setup Environment Variables

            ```bash
            CHART_VERSION=0.2.4+3003
            BUILD_CHART_VERSION=0.1.0+7
            CLUSTER_CONFIG=sbtaylor@pyrsia-prod.us-east-1.eksctl.io
            kubectl config use-context ${CLUSTER_CONFIG}
            PYRSIA_NAMESPACE=pyrsia-node
            PYRSIA_KEYPAIR=prod_eks_ed25519
            ```

        - Deploy

            ```bash
            PATH=/usr/local/Cellar/openssl@3/3.0.7/bin:$PATH
            openssl pkey -in ${PYRSIA_KEYPAIR}.pem -out - -outform DER | tail -c +17 > ${PYRSIA_KEYPAIR}.ser
            openssl pkey -in ${PYRSIA_KEYPAIR}.pem -pubout -outform DER | tail -c +13 >> ${PYRSIA_KEYPAIR}.ser

            helm upgrade --install node1 -n ${PYRSIA_NAMESPACE} pyrsia-nightly/pyrsia-node --set keys.p2p=$(cat ${PYRSIA_KEYPAIR}.ser | base64) --set keys.blockchain=$(cat ${PYRSIA_KEYPAIR}.ser | base64) --set "domain=${PYRSIA_DOMAIN}" --set "bootdns=${PYRSIA_BOOTDNS}" --version "${CHART_VERSION}"
            helm upgrade build1 --install -n pyrsia-node pyrsia-nightly/pyrsia-build-service --set "bootdns=${PYRSIA_BOOTDNS}" --version "${BUILD_CHART_VERSION}"
            ```

## Using helm to deploy updates

The helm charts are automatically published by the Github Actions to the <http://helmrepo.pyrsia.io> site. [ArtifactHub](https://artifacthub.io) pulls in chart updates every half hour.

Details about the Chart Values and Installation steps are documented in the chart's ReadMe.  ArtifactHub renders the ReadMe on the [Prysia Package](https://artifacthub.io/packages/helm/pyrsia-nightly/pyrsia-node) site.

1. Add the remote repo location to helm
`helm repo add pyrsia-nightly https://helmrepo.pyrsia.io/repos/nightly/`

2. Fetch the latest charts
`helm repo update`

3. Set you cluster connection
`kubectl config use-context <context name>`

4. Deployment

    - Staging for GKE
        - Setup Environment Variables

            ```bash
            CHART_VERSION=0.2.4+3003
            BUILD_CHART_VERSION=0.1.0+7
            CLUSTER_CONFIG=gke_pyrsia-sandbox_us-central1_pyrsia-staging
            kubectl config use-context ${CLUSTER_CONFIG}
            PYRSIA_NAMESPACE=pyrsia-node
            PYRSIA_BASE_DOMAIN=pyrsia.link
            PYRSIA_DOMAIN=staging.${PYRSIA_BASE_DOMAIN}
            PYRSIA_BOOTDNS=boot.${PYRSIA_DOMAIN}
            PYRSIA_P2P_KEYPAIR=$(kubectl get secret pyrsia-keys -n ${PYRSIA_NAMESPACE} -o json | jq -r '.data."ed25519.ser"')
            PYRSIA_BLOCKCHAIN_KEYPAIR=$(kubectl get secret pyrsia-keys -n ${PYRSIA_NAMESPACE} -o json | jq -r '.data."blockchain_ed25519.ser"')
            ```

        - Deploy

            ```bash
            helm upgrade --install node1 -n "${PYRSIA_NAMESPACE}" pyrsia-nightly/pyrsia-node --set "domain=${PYRSIA_DOMAIN}" --set "bootdns=${PYRSIA_BOOTDNS}" --set "keys.p2p=${PYRSIA_P2P_KEYPAIR}" --set "keys.blockchain=${PYRSIA_P2P_KEYPAIR}" --version "${CHART_VERSION}"
            helm upgrade build1 --install -n pyrsia-node pyrsia-nightly/pyrsia-build-service --set "bootdns=${PYRSIA_BOOTDNS}" --version "${BUILD_CHART_VERSION}"
            ```

    - Staging for EKS
        - Setup Environment Variables

            ```bash
            CHART_VERSION=0.2.4+3003
            BUILD_CHART_VERSION=0.1.0+7
            CLUSTER_CONFIG=sbtaylor@pyrsia-staging.us-east-1.eksctl.io
            kubectl config use-context ${CLUSTER_CONFIG}
            PYRSIA_NAMESPACE=pyrsia-node
            PYRSIA_BASE_DOMAIN=pyrsia-aws.link
            PYRSIA_DOMAIN=staging.${PYRSIA_BASE_DOMAIN}
            PYRSIA_BOOTDNS=boot.${PYRSIA_DOMAIN}
            PYRSIA_P2P_KEYPAIR=$(kubectl get secret pyrsia-keys -n ${PYRSIA_NAMESPACE} -o json | jq -r '.data."ed25519.ser"')
            PYRSIA_BLOCKCHAIN_KEYPAIR=$(kubectl get secret pyrsia-keys -n ${PYRSIA_NAMESPACE} -o json | jq -r '.data."blockchain_ed25519.ser"')
            ```

        - Deploy

            ```bash
            helm upgrade --install node1 -n "${PYRSIA_NAMESPACE}" pyrsia-nightly/pyrsia-node  --set "domain=${PYRSIA_DOMAIN}" --set "bootdns=${PYRSIA_BOOTDNS}" --set "keys.p2p=${PYRSIA_P2P_KEYPAIR}" --set "keys.blockchain=${PYRSIA_P2P_KEYPAIR}" --version "${CHART_VERSION}"
            helm upgrade build1 --install -n pyrsia-node pyrsia-nightly/pyrsia-build-service --set "bootdns=${PYRSIA_BOOTDNS}" --version "${BUILD_CHART_VERSION}"
            ```

    - Staging for GKE from branch

        - Setup Environment Variables

            ```bash
            CHART_VERSION=0.2.4+3003
            BUILD_CHART_VERSION=0.1.0+7
            CLUSTER_CONFIG=gke_pyrsia-sandbox_us-central1_pyrsia-staging
            kubectl config use-context ${CLUSTER_CONFIG}
            PYRSIA_NAMESPACE=pyrsia-node
            PYRSIA_BASE_DOMAIN=pyrsia.link
            PYRSIA_DOMAIN=staging.${PYRSIA_BASE_DOMAIN}
            PYRSIA_BOOTDNS=boot.${PYRSIA_DOMAIN}
            PYRSIA_P2P_KEYPAIR=$(kubectl get secret pyrsia-keys -n ${PYRSIA_NAMESPACE} -o json | jq -r '.data."ed25519.ser"')
            PYRSIA_BLOCKCHAIN_KEYPAIR=$(kubectl get secret pyrsia-keys -n ${PYRSIA_NAMESPACE} -o json | jq -r '.data."blockchain_ed25519.ser"')
            IMAGE_REPO=mydockerrepo/pyrsia
            IMAGE_TAG=1.0
            ```

        - Deploy

            From the root of your Pyrsia repo:

            ```bash
            docker login
            docker build --tag ${IMAGE_REPO}:${IMAGE_TAG}
            docker push ${IMAGE_REPO}:${IMAGE_TAG}

            helm upgrade --install node1 -n "${PYRSIA_NAMESPACE}" pyrsia-nightly/pyrsia-node --set "domain=${PYRSIA_DOMAIN}" --set "bootdns=${PYRSIA_BOOTDNS}" --set "keys.p2p=${PYRSIA_P2P_KEYPAIR}" --set "keys.blockchain=${PYRSIA_P2P_KEYPAIR}" --version "${CHART_VERSION}" --set "image.repository=${IMAGE_REPO}" --set "image.tag=${IMAGE_TAG}"
            helm upgrade build1 --install -n pyrsia-node pyrsia-nightly/pyrsia-build-service --set "bootdns=${PYRSIA_BOOTDNS}" --version "${BUILD_CHART_VERSION}"
            ```

    - Staging for EKS from branch

        - Setup Environment Variables

            ```bash
            CHART_VERSION=0.2.4+3003
            BUILD_CHART_VERSION=0.1.0+7
            CLUSTER_CONFIG=sbtaylor@pyrsia-staging.us-east-1.eksctl.io
            kubectl config use-context ${CLUSTER_CONFIG}
            PYRSIA_NAMESPACE=pyrsia-node
            PYRSIA_BASE_DOMAIN=pyrsia.link
            PYRSIA_DOMAIN=staging.${PYRSIA_BASE_DOMAIN}
            PYRSIA_BOOTDNS=boot.${PYRSIA_DOMAIN}
            PYRSIA_P2P_KEYPAIR=$(kubectl get secret pyrsia-keys -n ${PYRSIA_NAMESPACE} -o json | jq -r '.data."ed25519.ser"')
            PYRSIA_BLOCKCHAIN_KEYPAIR=$(kubectl get secret pyrsia-keys -n ${PYRSIA_NAMESPACE} -o json | jq -r '.data."blockchain_ed25519.ser"')
            IMAGE_REPO=mydockerrepo/pyrsia
            IMAGE_TAG=1.0
            ```

        - Deploy

            From the root of your Pyrsia repo:

            ```bash
            docker login
            docker build --tag ${IMAGE_REPO}:${IMAGE_TAG}
            docker push ${IMAGE_REPO}:${IMAGE_TAG}

            helm upgrade --install node1 -n "${PYRSIA_NAMESPACE}" pyrsia-nightly/pyrsia-node --set "domain=${PYRSIA_DOMAIN}" --set "bootdns=${PYRSIA_BOOTDNS}" --set "keys.p2p=${PYRSIA_P2P_KEYPAIR}" --set "keys.blockchain=${PYRSIA_P2P_KEYPAIR}" --version "${CHART_VERSION}" --set "image.repository=${IMAGE_REPO}" --set "image.tag=${IMAGE_TAG}"
            helm upgrade build1 --install -n pyrsia-node pyrsia-nightly/pyrsia-build-service --set "bootdns=${PYRSIA_BOOTDNS}" --version "${BUILD_CHART_VERSION}"
            ```

    - Production for GKE

        - Setup Environment Variables

            ```bash
            CHART_VERSION=0.2.4+3003
            BUILD_CHART_VERSION=0.1.0+7
            CLUSTER_CONFIG=gke_pyrsia-sandbox_us-central1-c_pyrsia-cluster-1
            kubectl config use-context ${CLUSTER_CONFIG}
            PYRSIA_NAMESPACE=pyrsia-node
            PYRSIA_P2P_KEYPAIR=$(kubectl get secret pyrsia-keys -n ${PYRSIA_NAMESPACE} -o json | jq -r '.data."ed25519.ser"')
            PYRSIA_BLOCKCHAIN_KEYPAIR=$(kubectl get secret pyrsia-keys -n ${PYRSIA_NAMESPACE} -o json | jq -r '.data."blockchain_ed25519.ser"')
            ```

        - Deploy

            ```bash
            helm upgrade --install node1 -n ${PYRSIA_NAMESPACE} pyrsia-nightly/pyrsia-node --set "keys.p2p=${PYRSIA_P2P_KEYPAIR}" --set "keys.blockchain=${PYRSIA_P2P_KEYPAIR}" --version "${CHART_VERSION}"
            helm upgrade build1 --install -n pyrsia-node pyrsia-nightly/pyrsia-build-service --version "${BUILD_CHART_VERSION}"
            ```

    - Production for EKS

        - Setup Environment Variables

            ```bash
            CHART_VERSION=0.2.4+3003
            BUILD_CHART_VERSION=0.1.0+7
            CLUSTER_CONFIG=sbtaylor@pyrsia-prod.us-east-1.eksctl.io
            kubectl config use-context ${CLUSTER_CONFIG}
            PYRSIA_NAMESPACE=pyrsia-node
            PYRSIA_BASE_DOMAIN=pyrsia-aws.link
            PYRSIA_DOMAIN=${PYRSIA_BASE_DOMAIN}
            PYRSIA_BOOTDNS=boot.${PYRSIA_DOMAIN}
            PYRSIA_P2P_KEYPAIR=$(kubectl get secret pyrsia-keys -n ${PYRSIA_NAMESPACE} -o json | jq -r '.data."ed25519.ser"')
            PYRSIA_BLOCKCHAIN_KEYPAIR=$(kubectl get secret pyrsia-keys -n ${PYRSIA_NAMESPACE} -o json | jq -r '.data."blockchain_ed25519.ser"')
            ```

        - Deploy

            ```bash
            helm upgrade --install node1 -n ${PYRSIA_NAMESPACE} pyrsia-nightly/pyrsia-node --set "domain=${PYRSIA_DOMAIN}" --set "keys.p2p=${PYRSIA_P2P_KEYPAIR}" --set "keys.blockchain=${PYRSIA_P2P_KEYPAIR}" --version "${CHART_VERSION}"
            helm upgrade build1 --install -n pyrsia-node pyrsia-nightly/pyrsia-build-service --version "${BUILD_CHART_VERSION}"
            ```

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
