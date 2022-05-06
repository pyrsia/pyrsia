#!/bin/bash

echo "Cleaning up previous runs"
docker-compose -f docker-compose-it.yml -p pyrsia-it down
docker volume rm pyrsia-it_certs-client1 pyrsia-it_certs-client2

echo "Starting two docker daemons"
docker-compose -f docker-compose-it.yml -p pyrsia-it up -d --build docker1 docker2
echo "Waiting for docker daemons"
w1=0
while [ $w1 -ne 2 ]; do
    sleep 5
    w1=$(docker-compose -f docker-compose-it.yml -p pyrsia-it logs docker1 docker2 | grep -F "API listen on [::]:2376" | wc -l)
done

echo "Starting pyrsia node 1"
docker-compose -f docker-compose-it.yml -p pyrsia-it up -d --build pyrsia1
echo "Waiting for pyrsia node 1"
w2=0
while [ $w2 -ne 1 ]; do
    sleep 5
    w2=$(docker-compose -f docker-compose-it.yml -p pyrsia-it logs pyrsia1 | grep -F "> Listen for p2p events" | wc -l)
done

echo "Starting pyrsia node 2"
docker-compose -f docker-compose-it.yml -p pyrsia-it up -d --build pyrsia2
echo "Waiting for pyrsia node 2"
w3=0
while [ $w3 -ne 1 ]; do
    sleep 5
    w3=$(docker-compose -f docker-compose-it.yml -p pyrsia-it logs pyrsia2 | grep -F "> Listen for p2p events" | wc -l)
done

echo "Running integration test"
docker-compose -f docker-compose-it.yml -p pyrsia-it up --build pyrsia-it

echo "Validating integration test"
v1=$(docker-compose -f docker-compose-it.yml -p pyrsia-it logs pyrsia2 | grep "Step 1: NO, alpine/3.15.4 does not exist in the metadata manager\\." | wc -l)
v2=$(docker-compose -f docker-compose-it.yml -p pyrsia-it logs pyrsia2 | grep "Step 2: YES, alpine/3.15.4 exists in the Pyrsia network\\." | wc -l)
v3=$(docker-compose -f docker-compose-it.yml -p pyrsia-it logs pyrsia2 | grep "Step 2: alpine/3.15.4 successfully stored locally from Pyrsia network\\." | wc -l)
v4=$(docker-compose -f docker-compose-it.yml -p pyrsia-it logs pyrsia2 | grep "Step 1: NO, \"sha256:df9b9388f04ad6279a7410b85cedfdcb2208c0a003da7ab5613af71079148139\" does not exist in the artifact manager\\." | wc -l)
v5=$(docker-compose -f docker-compose-it.yml -p pyrsia-it logs pyrsia2 | grep "Step 1: NO, \"sha256:0ac33e5f5afa79e084075e8698a22d574816eea8d7b7d480586835657c3e1c8b\" does not exist in the artifact manager\\." | wc -l)
v6=$(docker-compose -f docker-compose-it.yml -p pyrsia-it logs pyrsia2 | grep "Step 2: YES, \"sha256:df9b9388f04ad6279a7410b85cedfdcb2208c0a003da7ab5613af71079148139\" exists in the Pyrsia network\\." | wc -l)
v7=$(docker-compose -f docker-compose-it.yml -p pyrsia-it logs pyrsia2 | grep "Step 2: YES, \"sha256:0ac33e5f5afa79e084075e8698a22d574816eea8d7b7d480586835657c3e1c8b\" exists in the Pyrsia network\\." | wc -l)
v8=$(docker-compose -f docker-compose-it.yml -p pyrsia-it logs pyrsia2 | grep "Step 2: \"sha256:df9b9388f04ad6279a7410b85cedfdcb2208c0a003da7ab5613af71079148139\" successfully stored locally from Pyrsia network\\." | wc -l)
v9=$(docker-compose -f docker-compose-it.yml -p pyrsia-it logs pyrsia2 | grep "Step 2: \"sha256:0ac33e5f5afa79e084075e8698a22d574816eea8d7b7d480586835657c3e1c8b\" successfully stored locally from Pyrsia network\\." | wc -l)
if [[ $v1 -ne 1 || $v2 -ne 1 || $v3 -ne 1 || $v4 -ne 1 || $v5 -ne 1 || $v6 -ne 1 || $v7 -ne 1 || $v8 -ne 1 || $v9 -ne 1 ]]; then
    echo "[FAILURE] Pyrsia integration test failed. Expected lines missing: [$v1, $v2, $v3, $v4, $v5, $v6, $v7, $v8, $v9]"
else
    echo "[SUCCESS] Pyrsia integration test completed successfully!"
fi