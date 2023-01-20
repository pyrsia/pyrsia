# AWS EKS Setup

1. Generate Access keys for CLI, SDK, & API access

   - [Create Access Key](https://us-east-1.console.aws.amazon.com/iam/home?region=us-east-1#/security_credentials)
      - `aws configure`
         - Set `AWS Access Key ID`
         - Set `AWS Secret Access Key`
         - Set `Default region name`

2. [Install eksctl](https://docs.aws.amazon.com/eks/latest/userguide/eksctl.html)

3. Create the Cluster

   - See [cluster.yaml](cluster.yaml)
   - `eksctl create cluster -f cluster.yaml`

4. Create Kubernetes Namespaces
      - `kubectl create namespace pyrsia-node`
      - `kubectl create namespace external-dns`

5. Create Route 53 Policy
      - See [route53-policy.json](route53-policy.json)
      - `aws iam create-policy --policy-name "AllowExternalDNSUpdates" --policy-document file://route53-policy.json`

6. Attach Route 53 Policy
      - `aws iam attach-role-policy --role-name $(aws eks describe-nodegroup --cluster-name pyrsianode --nodegroup-name ng-1 --query nodegroup.nodeRole --out text | awk -F/ '{print $2}') --policy-arn $(aws iam list-policies --query 'Policies[?PolicyName==`AllowExternalDNSUpdates`].Arn' --output text)`

7. Setup Route 53 Domain
      - `aws route53 create-hosted-zone --name "pyrsia-aws.link." --caller-reference "external-dns-$(date +%s)"`

8. Get DNS Server List
      - `aws route53 list-resource-record-sets --output text --hosted-zone-id $(aws route53 list-hosted-zones-by-name --output json --dns-name "pyrsia-aws.link." | jq -r ".HostedZones[0].Id") --query "ResourceRecordSets[?Type == 'NS'].ResourceRecords[*].Value | []" | tr '\t' '\n'`

9. Generate Pyrsia Keys using openssl v3

   ```bash
   openssl genpkey -algorithm Ed25519 -out ed25519.pem
   openssl pkey -in ed25519.pem -pubout -outform DER | tail -c +13 > id_ed25519.pub
   openssl pkey -in ed25519.pem -out - -outform DER | tail -c +17 > id_ed25519.pri
   cat id_ed25519.pri id_ed25519.pub > ed25519.ser
   ```

10. Create DNS Alias
      - route53-alias.json

      ```bash

      aws route53 change-resource-record-sets \
      --hosted-zone-id $(aws route53 list-hosted-zones-by-name --output json --dns-name "pyrsia-aws.link." | jq -r ".HostedZones[0].Id" | cut -d/ -f3) \
      --change-batch '
      {
         "Comment": "Creating Alias resource for boot.nightly.pyrsia-aws.link",
         "Changes": [
            {
               "Action": "CREATE",
               "ResourceRecordSet": {
               "Name": "boot.nightly.pyrsia-aws.link",
               "Type": "A",
               "AliasTarget": {
                  "DNSName": "pyrsia-node-0.nightly.pyrsia-aws.link",
                  "EvaluateTargetHealth": false,
                  "HostedZoneId": "'$(aws route53 list-hosted-zones-by-name --output json --dns-name "pyrsia-aws.link." | jq -r ".HostedZones[0].Id" | cut -d/ -f3 )'"
               }
               }
            }
         ]
      }'
      ```

11. Deploy Pyrsia via Helm
      - `helm repo update pyrsia-nightly`
      - `helm upgrade node1 --install -n pyrsia-node pyrsia-nightly/pyrsia-node --set k8s_provider=eks --set "dnsname=nightly.pyrsia-aws.link" --set bootdns=boot.nightly.pyrsia-aws.link --set keys.p2p=$(cat ed25519.ser | base64) --set keys.blockchain=$(cat ed25519.ser | base64)  --version "0.2.4+2856`

      > Note: The above helm command does not setup the Pyrsia Node to use a Build Node.  `--set "buildnode=http://35.193.148.20:8080"` parameter is needed for build node configuraion.
