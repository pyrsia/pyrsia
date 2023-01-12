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

# We need to split out build commands into separate calls in order to run them in parallel
# therefore we cannot use --all-targets since that is equivalent to --lib --bins --tests --benches --examples
# which disables the parallelizm we are trying achive for speed. 
#
# "cargo build --workspace"  is equivalent to cargo build --workspace --lib --bins
# adding --lib --bins for clarity
#

$( cargo build --workspace --lib --bins --release; echo $? > /tmp/ws.rc ) &
$( cargo build --workspace --tests --benches --examples --release 1>/tmp/tests.log 2>&1; echo $? > /tmp/tests.rc ) &
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

# Check if OpenSSL is back
if [[ $(find . -name "Cargo.lock" -exec grep -i openssl {} \; | wc -l) -ne 0 ]]; then
    echo "OpenSSL Presence detected in the Cargo; please remove it and rebuild. Dumping Cargo.lock files to log."
    find . -name "Cargo.lock" -exec cat {} \;
    cargo tree
    exit 1
fi

# Return the max return code between the two processes.
# This is done to tell the GitHub Step to fail (rc != 0). The 
# value of the rc is not important at this point since each
# build has already displayed its return code. 

rc=$(cat /tmp/ws.rc /tmp/tests.rc | sort -nr | head -n1)

exit $rc
