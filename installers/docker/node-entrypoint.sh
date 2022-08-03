#!/usr/bin/env bash
set -e

# determine if we are running under kubernetes
if [ -f /var/run/secrets/kubernetes.io/serviceaccount/namespace ]; then
    APISERVER=https://kubernetes.default.svc
    SERVICEACCOUNT=/var/run/secrets/kubernetes.io/serviceaccount
    NAMESPACE=$(cat ${SERVICEACCOUNT}/namespace)
    TOKEN=$(cat ${SERVICEACCOUNT}/token)
    CACERT=${SERVICEACCOUNT}/ca.crt
    PODNAME=`hostname`
    export PYRSIA_EXTERNAL_IP=$(curl -s --cacert ${CACERT} --header "Authorization: Bearer ${TOKEN}" -X GET ${APISERVER}/api/v1/namespaces/${NAMESPACE}/services/${PODNAME} | jq -r ".status.loadBalancer.ingress[0].ip")
    echo "PYRSIA_EXTERNAL_IP=$PYRSIA_EXTERNAL_IP"

    # detemine if I am node-0 if so be the primary boot node
    if [ "$HOSTNAME" == "pyrsia-node-0" ]; then
        /usr/bin/pyrsia_node $* 
    else
        # I am not node-0 so find node-0 boot address and connect to it
        BOOTADDR=$(curl -s http://pyrsia-node-0.pyrsia.link/status | jq -r ".peer_addrs[0]")
        /usr/bin/pyrsia_node $* -P $BOOTADDR
    fi
else
    # not running under k8s so just startup in default mode
    BOOTADDR=$(curl -s http://boot.pyrsia.link/status | jq -r ".peer_addrs[0]")
    /usr/bin/pyrsia_node $* -P $BOOTADDR
fi
