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
    if [ "$(hostname | rev | cut -c -2 | rev)" = "-0" ]; then
        /usr/bin/pyrsia_node $* --listen-only true  
    else
        /usr/bin/pyrsia_node $* 
    fi
else
    /usr/bin/pyrsia_node $* 
fi
