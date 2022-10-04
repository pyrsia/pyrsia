#!/usr/bin/env bash

set -x

# determine if we are running under kubernetes
if [ -f /var/run/secrets/kubernetes.io/serviceaccount/namespace ]; then
    APISERVER=https://kubernetes.default.svc
    SERVICEACCOUNT=/var/run/secrets/kubernetes.io/serviceaccount
    NAMESPACE=$(cat ${SERVICEACCOUNT}/namespace)
    TOKEN=$(cat ${SERVICEACCOUNT}/token)
    CACERT=${SERVICEACCOUNT}/ca.crt
    PODNAME=`hostname`
    export PYRSIA_EXTERNAL_IP=$(curl -s --cacert ${CACERT} --header "Authorization: Bearer ${TOKEN}" -X GET ${APISERVER}/api/v1/namespaces/${NAMESPACE}/services/${PODNAME} | jq -r ".status.loadBalancer.ingress[0].ip")

    # Wait for the service to be assigned an ip addr
    while [ "${PYRSIA_EXTERNAL_IP}" == "" ] || [ "${PYRSIA_EXTERNAL_IP}" == "null" ]; do
        sleep 5
        export PYRSIA_EXTERNAL_IP=$(curl -s --cacert ${CACERT} --header "Authorization: Bearer ${TOKEN}" -X GET ${APISERVER}/api/v1/namespaces/${NAMESPACE}/services/${PODNAME} | jq -r ".status.loadBalancer.ingress[0].ip")
    done
    echo "PYRSIA_EXTERNAL_IP=$PYRSIA_EXTERNAL_IP"

    NODE_DOMAIN=$(echo $PYRSIA_BOOTDNS | cut -d'.' -f2-)
    NODE_HOSTNAME=${HOSTNAME}.${NODE_DOMAIN}
 
    # Wait for the ip addr to be mapped to dns and propogated

    DNSREADY=$(dig ${NODE_HOSTNAME} 2>&1 | grep "${PYRSIA_EXTERNAL_IP}")
    while [ "${DNSREADY}" == ""  ]; do
        sleep 5
        DNSREADY=$(dig ${NODE_HOSTNAME} 2>&1 | grep "${PYRSIA_EXTERNAL_IP}")
    done

    # detemine if I am pyrsia-node-0.pyrsia.link (first boot node ever) if so be the primary boot node

    echo Find Boot Zero
    echo ${DNSREADY}
    echo DONE

    BOOTZERO=$(echo "${DNSREADY}" | grep "${PYRSIA_BOOTDNS}")
    if  [ "${BOOTZERO}" != "" ]; then  
        /usr/bin/pyrsia_node $* --listen-only --init-blockchain
        exit $?
    fi
fi

# I am not node-0 so use boot.pyrsia.link for boot address and connect to it

# Wait for the status from node-0 to be available
curl http://${PYRSIA_BOOTDNS}/status

BOOTADDR=$(curl -s http://${PYRSIA_BOOTDNS}/status | jq -r ".peer_addrs[0]")

while [ "${BOOTADDR}" == "" ] || [ "${BOOTADDR}" == "null" ]; do
    sleep 5
    BOOTADDR=$(curl -s http://${PYRSIA_BOOTDNS}/status | jq -r ".peer_addrs[0]")
done

/usr/bin/pyrsia_node $* -P $BOOTADDR
