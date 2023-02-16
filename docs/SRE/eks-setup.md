# AWS EKS Setup

1. Generate Access keys for CLI, SDK, & API access

   - [Create Access Key](https://us-east-1.console.aws.amazon.com/iam/home?region=us-east-1#/security_credentials)
      - `aws configure`
         - Set `AWS Access Key ID`
         - Set `AWS Secret Access Key`
         - Set `Default region name`

2. [Install eksctl](https://docs.aws.amazon.com/eks/latest/userguide/eksctl.html)

3. Setup Environment Variables

   ```bash
   CHART_VERSION=0.2.4+3003
   BUILD_CHART_VERSION=0.1.0+7
   CLUSTER_NAME=pyrsia-staging
   EXTERNALDNS_NAMESPACE=external-dns
   PYRSIA_NAMESPACE=pyrsia-node
   PYRSIA_BASE_DOMAIN=pyrsia-aws.link
   PYRSIA_DOMAIN=staging.${PYRSIA_BASE_DOMAIN}
   PYRSIA_BOOTDNS=boot.${PYRSIA_DOMAIN}
   PYRSIA_NODE_ZERO=pyrsia-node-0.${PYRSIA_DOMAIN}
   ```

4. Create the Cluster

   ```bash
   cat <<EOF | eksctl create cluster -f -
   ---
   apiVersion: eksctl.io/v1alpha5
   kind: ClusterConfig
   metadata:
     name: ${CLUSTER_NAME}
     region: us-east-1
   cloudWatch:
     clusterLogging:
       enableTypes:
         - audit
         - authenticator
   managedNodeGroups:
     - name: ng-1
       amiFamily: AmazonLinux2
       instanceSelector:
         cpuArchitecture: x86_64
         memory: 2GiB
         vCPUs: 2
       instanceTypes:
         - t3.small
         - t3a.small
   iam:
     withOIDC: true
   addons:
     - name: aws-ebs-csi-driver
       version: v1.13.0-eksbuild.3
       attachPolicyARNs:
         - arn:aws:iam::aws:policy/service-role/AmazonEBSCSIDriverPolicy
   EOF
   ```

5. Create Kubernetes Namespaces
      - `kubectl create namespace ${PYRSIA_NAMESPACE}`
      - `kubectl create namespace ${EXTERNALDNS_NAMESPACE}`

6. Create Route 53 Policy
      - See [route53-policy.json](route53-policy.json)
      - `aws iam create-policy --policy-name "AllowExternalDNSUpdates" --policy-document file://route53-policy.json`

7. Attach Route 53 Policy
      - `aws iam attach-role-policy --role-name $(aws eks describe-nodegroup --cluster-name ${CLUSTER_NAME} --nodegroup-name ng-1 --query nodegroup.nodeRole --out text | awk -F/ '{print $2}') --policy-arn $(aws iam list-policies --query 'Policies[?PolicyName==`AllowExternalDNSUpdates`].Arn' --output text)`

8. Setup Route 53 Domain
      - `aws route53 create-hosted-zone --name "${PYRSIA_BASE_DOMAIN}." --caller-reference "external-dns-$(date +%s)"`

9. Get DNS Server List
      - `aws route53 list-resource-record-sets --output text --hosted-zone-id $(aws route53 list-hosted-zones-by-name --output json --dns-name "${PYRSIA_BASE_DOMAIN}." | jq -r ".HostedZones[0].Id") --query "ResourceRecordSets[?Type == 'NS'].ResourceRecords[*].Value | []" | tr '\t' '\n'`

10. Generate Pyrsia Keys using openssl v3

      ```bash
      openssl genpkey -algorithm Ed25519 -out ed25519.pem
      openssl pkey -in ed25519.pem -pubout -outform DER | tail -c +13 > id_ed25519.pub
      openssl pkey -in ed25519.pem -out - -outform DER | tail -c +17 > id_ed25519.pri
      cat id_ed25519.pri id_ed25519.pub > ed25519.ser
      ```

11. Create DNS Alias

      ```bash

      aws route53 change-resource-record-sets \
      --hosted-zone-id $(aws route53 list-hosted-zones-by-name --output json --dns-name "${PYRSIA_BASE_DOMAIN}." | jq -r ".HostedZones[0].Id" | cut -d/ -f3) \
      --change-batch '
      {
         "Comment": "Creating Alias resource for '${PYRSIA_BOOTDNS}'",
         "Changes": [
            {
               "Action": "CREATE",
               "ResourceRecordSet": {
               "Name": "'${PYRSIA_BOOTDNS}'",
               "Type": "A",
               "AliasTarget": {
                  "DNSName": "'${PYRSIA_NODE_ZERO}'",
                  "EvaluateTargetHealth": false,
                  "HostedZoneId": "'$(aws route53 list-hosted-zones-by-name --output json --dns-name "${PYRSIA_BASE_DOMAIN}}." | jq -r ".HostedZones[0].Id" | cut -d/ -f3 )'"
               }
               }
            }
         ]
      }'
      ```

12. Deploy Pyrsia via Helm

      - `helm repo update pyrsia-nightly`
      - `helm upgrade node1 --install -n pyrsia-node pyrsia-staging/pyrsia-node --set "domain=${PYRSIA_DOMAIN}" --set bootdns=${PYRSIA_BOOTDNS} --set keys.p2p=$(cat ed25519.ser | base64) --set keys.blockchain=$(cat ed25519.ser | base64)  --version "${CHART_VERSION}"`

13. Deploy Build Service via Helm (Optional)

      - `helm upgrade build1 --install -n pyrsia-node pyrsia-nightly/pyrsia-build-service --set bootdns=${PYRSIA_BOOTDNS} --version "${BUILD_ChART_VERSION}"`
