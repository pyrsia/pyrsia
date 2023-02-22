#!/usr/bin/env bash

####################### FUNCTIONS #######################################

# prints header
header() {
    echo "####################################################### ${1} #######################################################"
}

# print footer
footer() {
    echo "####################################################### ${1} #######################################################"
}

# Kills started processes
function kill_processes() {
    for pidfile in $TEST_DIR/*.pid; do
        read pid <$pidfile
        kill $pid
    done
}

# Prints message to std error, kills started processes and exit with error code
fatal()
{
  echo "fatal: $1" 1>&2
  kill_processes
  exit 1
}

# Listing of started pyrsia processes
function list_started_processes() {
    local pidlist=""
    for pidfile in $TEST_DIR/*.pid; do
        read pid <$pidfile
        if [ ! -z "$pidlist" ] ; then 
            pidlist="$pidlist,"; 
        fi
        pidlist="$pidlist$pid";
    done
    echo -e "\nPyrsia processes:"
    ps -p $pidlist
}

# Waits period of time until the peer's http status json-response contains peer_id
# Function params:
#   1) http port of peer
function wait_status_ok() {
    local port=$1
    
    local time_counter=$MAX_WAITING_TIME
    
    while [ $time_counter -ne 0 ]
    do
        local peer_id=`curl -s http://localhost:${port}/status | jq -r .peer_id`
        if [ -z "$peer_id" ]
        then
            sleep 1
            ((time_counter-=1))
        else
            break
        fi
    done

    if [ $time_counter -eq 0 ]
    then
        fatal "Port ${port} is not reachable"
    fi
}

# Waits period of time until text message be found in log file
# Function params:
#   1) Message for searching in log file
#   2) Path to log file
function wait_message_in_log() {
    local message=$1
    local log_file=$2    
    
    local time_counter=$MAX_WAITING_TIME
    time_counter=2
    
    while [ $time_counter -ne 0 ]
    do
        if grep -q "${message}" $log_file
        then
            break
        else
            sleep 1
            ((time_counter-=1))
        fi
    done

    if [ $time_counter -eq 0 ]
    then
        fatal "Cannot find '${message}' in file ${log_file}"
    fi
}

# Builds 'build pipeline' and starts it
function start_build_pipeline() {
    echo "'Build pipeline' is starting ..."

    cd $PYRSIA_BUILD_PIPELINE_HOME

    local pid_file="$TEST_DIR/build_pipeline.pid"
    local output_log="$TEST_DIR/build_pipeline.log"
    (RUST_LOG=debug cargo run &>${output_log}) &
    local pid=$!
    echo $pid >$pid_file

    wait_message_in_log "INFO  actix_server::server  > Tokio runtime found; starting in existing Tokio runtime" $output_log

    echo "'Build pipeline' is started sucessfully"
}

# Starts nodeA
function start_nodeA() {
    local pipeline_service_endpoint=$1
    local port=$2

    local node=nodeA

    echo "Starting authorizing node ${node}"
    local pid_file="$TEST_DIR/${node}.pid"
    local output_log="$TEST_DIR/${node}.log"
    cd $TEST_DIR/${node}
    (RUST_LOG=pyrsia=debug DEV_MODE=on ./pyrsia_node --pipeline-service-endpoint ${pipeline_service_endpoint}  --listen-only -H 0.0.0.0 -p ${port} --init-blockchain &>${output_log}) &
    local pid=$!
    echo $pid >$pid_file

    wait_status_ok $port
}

# Starts nodeB
function start_nodeB() {
    local bootstrap_url=$1
    local pipeline_service_endpoint=$2
    local port=$3

    local node=nodeB

    echo "Starting authorizing node ${node}"
    local pid_file="$TEST_DIR/${node}.pid"
    local output_log="$TEST_DIR/${node}.log"
    cd $TEST_DIR/${node}
    (RUST_LOG=debug ./pyrsia_node --bootstrap-url ${bootstrap_url} --pipeline-service-endpoint ${pipeline_service_endpoint} -p ${port} &>${output_log}) &
    local pid=$!
    echo $pid >$pid_file

    wait_status_ok $port
}

# starts regular node
function start_regular_node() {
    local node=$1
    local bootstrap_url=$2
    local port=$3

    echo "Starting regular ${node}"
    local pid_file="$TEST_DIR/${node}.pid"
    local output_log="$TEST_DIR/${node}.log"
    cd $TEST_DIR/${node}
    (RUST_LOG=debug ./pyrsia_node --bootstrap-url ${bootstrap_url} -p ${port} &>${output_log}) &
    local pid=$!
    echo $pid >$pid_file

    wait_status_ok $port
}

# Trigger a build from node
# Function params:
#   1) config_port - Node configuration port
#   2) docker_image - Docker image identifier for which a new build is reguired to trigger
function trigger_build() {
    local config_port=$1
    local docker_image=$2

    ./target/debug/pyrsia config -e --port ${config_port}
    sleep $SLEEP_DURATION
    local text=$(./target/debug/pyrsia build docker --image ${docker_image})
    echo "$text"
    sleep $SLEEP_DURATION

    local pattern="'(.*)'"
    if  [[ $text =~ $pattern ]]; then
        local build_id=${BASH_REMATCH[1]}
    else
        fatal "Cannot parse Build ID in '${text}'"
    fi

    echo "BUILD ID is '${build_id}'"
}

# Checks(inspects) logs on all nodes for docker image.
# Function params:
#   1) docker_image - Docker image identifier to check logs.
function inspect_logs_of_all_nodes() {
    local docker_image=$1

    for port in 7881 7882 7883 7884
    do
        ./target/debug/pyrsia config -e --port ${port}
        sleep 3

        local time_counter=$MAX_WAITING_TIME
    
        while [ $time_counter -ne 0 ]
        do
            local text=$(./target/debug/pyrsia inspect-log docker --image ${docker_image})
            if  [[ $text =~ 'artifact_hash' ]]; then
                break
            else
                sleep 1
                ((time_counter-=1))
            fi
        done

        if [ $time_counter -eq 0 ]
        then
            fatal "Cannot find artifact info on node by port ${port}"
        fi

        echo "The transparency log contains artifact info on node by port ${port}"
    done
}

# Check if the artifact is accessible on all nodes.
# Function params:
#   1) docker_image_version - Docker image identifier version to check accessible.
function artifact_accessible_on_all_nodes() {
    local docker_image_version=$1

    for port in 7881 7882 7883 7884
    do
        local url=http://0.0.0.0:${port}/v2/library/alpine/manifests/${docker_image_version}
        local text=$(curl $url)
        if  [[ $text =~ 'errors' ]]; then
            echo "fatal"
            exit 1
        fi
        echo "Artifact is accessible on node by address ${url}"
    done
}

# absolute path of directory
function abs_path() {
    local path="$1"
    if [ ! ${path:0:1} = '/' ]
    then
        path="$PWD/$path"
    fi
    echo "$path"
}

####################### CONSTANTS #######################################

# This constants specifies maximum period of awating something and is used in functions that await some condition, for example show up message in some file
MAX_WAITING_TIME=5

# sleep duration
SLEEP_DURATION=3

# working directory for test
TEST_DIR=/tmp/pyrsia-manual-tests

####################### MAIN SCENARIO #######################################

echo "##############################################################################################################"
echo "#"
echo "# Integration test including multiple authorized nodes"
echo "#"
header

#set -x
set -e

if [ $# -ne 2 ] 
then
    echo "Usage: `basename $0` pyrsia_home pyrsia_build_pipeline_home"
    exit 1
fi

PYRSIA_HOME=$(abs_path $1)
PYRSIA_BUILD_PIPELINE_HOME=$(abs_path $2)

if [ ! -e $PYRSIA_HOME ]; then
    fatal "Cannot find pyrsia home by path: $PYRSIA_HOME"
fi

if [ ! -e $PYRSIA_BUILD_PIPELINE_HOME ]; then
    fatal "Cannot find pyrsia build pipeline home by path: $PYRSIA_BUILD_PIPELINE_HOME"
fi

echo "PYRSIA_HOME=${PYRSIA_HOME}"
echo "PYRSIA_BUILD_PIPELINE_HOME=${PYRSIA_BUILD_PIPELINE_HOME}"

{
    echo
    header "STEP 0 (START NODES)"

    echo "Building of pyrsia is starting"
    cd ${PYRSIA_HOME}
    cargo build --workspace
    echo "Pyrsia is built successfully"

    if [[ -d $TEST_DIR ]]
    then
        rm -rf $TEST_DIR
    fi
    for node in nodeA nodeB nodeC nodeD
    do
        dir=${TEST_DIR}/${node}
        mkdir -p $dir || fatal "Could not create directory: \"${dir}\""
        cp ${PYRSIA_HOME}/target/debug/pyrsia_node ${TEST_DIR}/${node}
    done

    start_build_pipeline
    start_nodeA "http://localhost:8080" 7881
    start_nodeB "http://localhost:7881/status" "http://localhost:8080" 7882
    start_regular_node nodeC "http://localhost:7881/status" 7883
    start_regular_node nodeD "http://localhost:7882/status" 7884

    list_started_processes
    echo

    footer "STEP 0 - DONE"
}

{
    echo
    header "STEP 1 (set up the authorized nodes)"

    cd $PYRSIA_HOME
    NODE_A_PEER_ID=`curl -s http://localhost:7881/status | jq -r .peer_id`
    NODE_B_PEER_ID=`curl -s http://localhost:7882/status | jq -r .peer_id`
    echo "NODE_A_PEER_ID=$NODE_A_PEER_ID"
    echo "NODE_B_PEER_ID=$NODE_B_PEER_ID"
    ./target/debug/pyrsia config -e --port 7881

    sleep $SLEEP_DURATION
    text=$(./target/debug/pyrsia authorize --peer $NODE_A_PEER_ID)
    echo $text
    if  [[ $text =~ 'Authorize request successfully handled' ]]; then
        echo
    else
        fatal "Cannot authorize peer $NODE_A_PEER_ID"
    fi

    sleep $SLEEP_DURATION
    text=$(./target/debug/pyrsia authorize --peer $NODE_B_PEER_ID)
    echo $text
    if  [[ $text =~ 'Authorize request successfully handled' ]]; then
        echo
    else
        fatal "Cannot authorize peer $NODE_B_PEER_ID"
    fi

    echo "nodeA and nodeB are authorized successfully"

    footer "STEP 1 - Done"
}

{
    echo
    header "STEP 2 (Trigger a build from node A)"
    trigger_build 7881 'alpine:3.16.0'
    footer "STEP 2 - Done"
}

{
    echo
    header "STEP 3 (Wait for the build to finish and check logs on all nodes)"
    inspect_logs_of_all_nodes 'alpine:3.16.0'
    footer "STEP 3 - Done"
}

{
    echo
    header "STEP 4 (check if the artifacts itself are accessible on all nodes)"
    artifact_accessible_on_all_nodes '3.16.0'
    footer "STEP 4 - Done"
}

{
    echo
    header "STEP 5 (check if a build by node B works in the same way)"
    trigger_build 7882 'alpine:3.16.1'
    footer "STEP 5 - Done"
}

{
    echo
    header "STEP 6 (Wait for the build to finish and check logs on all nodes)"
    inspect_logs_of_all_nodes 'alpine:3.16.1'
    footer "STEP 6 - Done"
}

{
    echo
    header "STEP 7 (check if the artifacts itself are accessible on all nodes)"
    artifact_accessible_on_all_nodes '3.16.1'
    footer "STEP 7 - Done"
}

kill_processes

echo ""; echo "All test scenarios are done successfully!"
