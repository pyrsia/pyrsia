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

    # detemine if I am pyrsia-node-0.pyrsia.link (first boot node ever) if so be the primary boot node
    # need to use wget since nslookup, dig and ping are not installed
    if wget --timeout=2 -v pyrsia-node-0.pyrsia.link 2>&1 | grep Connecting | grep -q "${PYRSIA_EXTERNAL_IP}" ; then  
        /usr/bin/pyrsia_node $* --listen-only true  
    else
        # I am not pyrsia-node-0.pyrsia.link so use boot.pyrsia.link for boot address and connect to it
        /usr/bin/pyrsia_node $*
    fi
else
    # not running under k8s so just startup in default mode
    /usr/bin/pyrsia_node $*
fi
