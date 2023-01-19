# Google GKE Setup

1. Generate Access keys for CLI, SDK, & API access

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

2. Create the Cluster

   - `gcloud container clusters create pyrsia-staging --logging=SYSTEM,API_SERVER --num-nodes=3 --enable-autoupgrade --machine-type=e2-standard-2 --region=us-central1 --preemptible --service-account=prysia-k8s@pyrsia-sandbox.iam.gserviceaccount.com`

3. Set kubectl config access

   - `gcloud container clusters get-credentials pyrsia-staging --zone=us-central1-c`

4. Create Kubernetes Namespaces
   - `kubectl create namespace pyrsia-node`
   - `kubectl create namespace external-dns`

5. Create DNS Zone

   - `gcloud dns managed-zones create pyrsia-link --project pyrsia-sandbox --description pyrsia.link --dns-name=pyrsia.link. --visibility=public`

6. Print list of name servers
   - `gcloud dns record-sets list --project pyrsia-sandbox --zone pyrsia-link --name "pyrsia.link." --type NS --format "value(rrdatas)" | tr ';' '\n'`

7. Create round robin DNS name
   - `gcloud dns --project=pyrsia-sandbox record-sets create boot.staging.pyrsia.link. --zone="pyrsia-link" --type="CNAME" --ttl="300" --routing-policy-type="WRR" --routing-policy-data="50.0=pyrsia-node-0.staging.pyrsia.link."`

8. Add DNS Admin to Service Account
   `gcloud projects add-iam-policy-binding pyrsia-sandbox --member serviceAccount:prysia-k8s@pyrsia-sandbox.iam.gserviceaccount.com --role roles/dns.admin`

9. Generate Pyrsia Keys using openssl v3

   ```bash
   /usr/local/Cellar/openssl@3/3.0.7/bin/openssl genpkey -algorithm Ed25519 -out ed25519.pem
   /usr/local/Cellar/openssl@3/3.0.7/bin/openssl pkey -in ed25519.pem -pubout -outform DER | tail -c +13 > id_ed25519.pub
   /usr/local/Cellar/openssl@3/3.0.7/bin/openssl pkey -in ed25519.pem -out - -outform DER | tail -c +17 > id_ed25519.pri
   cat id_ed25519.pri id_ed25519.pub > ed25519.ser
   ```

10. Deploy Pyrsia via Helm
      - `helm repo update pyrsia-nightly`
      - `helm upgrade node1 --install -n pyrsia-node pyrsia-nightly/pyrsia-node --set k8s_provider=gke --set "dnsname=staging.pyrsia.link" --set bootdns=boot.staging.pyrsia.link --set keys.p2p=$(cat ed25519.ser | base64) --set keys.blockchain=$(cat ed25519.ser | base64) --version "0.2.4+2856`

      > Note: The above helm command does not setup the Pyrsia Node to use a Build Node.  `--set "buildnode=http://35.193.148.20:8080"` parameter is needed for build node configuraion.
