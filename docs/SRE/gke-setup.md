# Google GKE Setup

1. Generate Access keys for CLI, SDK, & API access

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

2. Setup Environment Variables

   ```bash
   CHART_VERSION=0.2.4+3003
   BUILD_CHART_VERSION=0.1.0+7
   CLUSTER_NAME=pyrsia-staging
   EXTERNALDNS_NAMESPACE=external-dns
   PYRSIA_NAMESPACE=pyrsia-node
   PYRSIA_BASE_DOMAIN=pyrsia.link
   PYRSIA_DOMAIN=staging.${PYRSIA_BASE_DOMAIN}
   PYRSIA_BOOTDNS=boot.${PYRSIA_DOMAIN}
   PYRSIA_NODE_ZERO=pyrsia-node-0.${PYRSIA_DOMAIN}
   SERVICE_ACCOUNT=prysia-k8s@pyrsia-sandbox.iam.gserviceaccount.com
   PROJECT=pyrsia-sandbox
   ```

3. Create the Cluster

   - `gcloud container clusters create ${CLUSTER_NAME} --logging=SYSTEM,API_SERVER --num-nodes=3 --enable-autoupgrade --machine-type=e2-standard-2 --region=us-central1 --preemptible --service-account=${SERVICE_ACCOUNT}`

4. Set kubectl config access

   - `gcloud container clusters get-credentials ${CLUSTER_NAME} --zone=us-central1-c`

5. Create Kubernetes Namespaces
   - `kubectl create namespace ${PYRSIA_NAMESPACE}`
   - `kubectl create namespace ${EXTERNALDNS_NAMESPACE}`

6. Create DNS Zone

   - `gcloud dns managed-zones create pyrsia-zone --project ${PROJECT} --description ${PYRSIA_BASE_DOMAIN} --dns-name=${PYRSIA_BASE_DOMAIN}. --visibility=public`

7. Print list of name servers
   - `gcloud dns record-sets list --project ${PROJECT} --zone pyrsia-zone --name "${PYRSIA_BASE_DOMAIN}." --type NS --format "value(rrdatas)" | tr ';' '\n'`

8. Create round robin DNS name
   - `gcloud dns --project=${PROJECT} record-sets create ${PYRSIA_BOOTDNS}. --zone=pyrsia-zone --type="CNAME" --ttl="300" --routing-policy-type="WRR" --routing-policy-data="50.0=${PYRSIA_NODE_ZERO}."`

9. Add DNS Admin to Service Account
   `gcloud projects add-iam-policy-binding ${PROJECT} --member serviceAccount:${SERVICE_ACCOUNT} --role roles/dns.admin`

10. Generate Pyrsia Keys using openssl v3

      ```bash
      /usr/local/Cellar/openssl@3/3.0.7/bin/openssl genpkey -algorithm Ed25519 -out ed25519.pem
      /usr/local/Cellar/openssl@3/3.0.7/bin/openssl pkey -in ed25519.pem -pubout -outform DER | tail -c +13 > id_ed25519.pub
      /usr/local/Cellar/openssl@3/3.0.7/bin/openssl pkey -in ed25519.pem -out - -outform DER | tail -c +17 > id_ed25519.pri
      cat id_ed25519.pri id_ed25519.pub > ed25519.ser
      ```

11. Deploy Pyrsia via Helm
      - `helm repo update pyrsia-nightly`
      - `helm upgrade node1 --install -n pyrsia-node pyrsia-nightly/pyrsia-node --set "domain=${PYRSIA_DOMAIN}" --set bootdns=${PYRSIA_BOOTDNS} --set keys.p2p=$(cat ed25519.ser | base64) --set keys.blockchain=$(cat ed25519.ser | base64) --version "${CHART_VERSION}"`

12. (Optional) Deploy Build Service via Helm
      - `helm upgrade build1 --install -n pyrsia-node pyrsia-nightly/pyrsia-build-service --set bootdns=${PYRSIA_BOOTDNS} --version "${BUILD_CHART_VERSION}"`
