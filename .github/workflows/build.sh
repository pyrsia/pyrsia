#!/usr/bin/env bash

# We need to run both the workspace and test build at the same time in
# the same GitHub action step.  This is done so the cache can be shared.
#
# GitHub action steps run serially and have a shared cache.  The GitHub
# action jobs can run in parallel but do not share the same cache.
#
# The workaround is to run the builds as background processes and wait
# for them to finish before exiting the script.

# Need to save the return codes from the background processes to exit the
# script with a bad return code if either fails

set -e

$( cargo build --workspace --verbose --release; echo $? > /tmp/ws.rc ) &
$( cargo build --tests --verbose --release 1>/tmp/tests.log 2>&1; echo $? > /tmp/tests.rc ) &
jobs
wait

# Display the workspace return code
echo "### Workspace Build RC=$(cat /tmp/ws.rc)"

# The workspace build is outputing as it runs so we just need to
# output the test build once they are both done running
echo cargo build --tests --verbose --release
cat /tmp/tests.log

# Display the tests return code
echo "### Tests Build RC=$(cat /tmp/tests.rc)"

# Return the max return code between the two processes.
# This is done to tell the GitHub Step to fail (rc != 0). The 
# value of the rc is not important at this point since each
# build has already displayed its return code. 

rc=$(cat /tmp/ws.rc /tmp/tests.rc | sort -nr | head -n1)
exit $rc
