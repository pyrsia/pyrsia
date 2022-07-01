# How to run 2 Pyrsia nodes natively on 1 MacOS machine

Download the release version of Pyrsia from [Release v0.1.0](https://github.com/pyrsia/pyrsia/releases/tag/v0.1.0)
Untar/Unzip the downloaded source code in a local folder.

Let's call this folder `PYRSIA_HOME`. We will refer to this name in the following steps.

Build binaries for `pyrsia_node` by running:

```sh
cd $PYRSIA_HOME
cargo build --workspace
```

## Create 2 separate nodes "installations"

This will create two copies of the same binary so that you can configure them
as independent nodes. In `PYRSIA_HOME`,

- Create Node A

  ```sh
  mkdir nodeA

  cp target/debug/pyrsia_node nodeA
  ```

- Create Node B

 ```sh
 mkdir nodeB

 cp target/debug/pyrsia_node nodeB
 ```

### Start Node A

In a new terminal start node A, http listen on 7888 and p2p listen on 44001

```sh
cd $PYRSIA_HOME
cd nodeA
DEV_MODE=on RUST_LOG="pyrsia=debug,info"  ./pyrsia_node -H 0.0.0.0 -p 7888 -L /ip4/0.0.0.0/tcp/44001
```

If everything goes well, you will see a line similar to the following in the
logs on the terminal (The IP address could be different than in the sample below):

```text
# INFO  pyrsia::network::p2p > Local node is listening on "/ip4/192.168.0.110/tcp/44001/p2p/12D3KooWLKMbBzp4k1mcM2rYXs8VQgoCSNLxGUwnB1itouxYcnx3"
```

If you do not find this line right away try with `grep 44001`

### Start Node B

In a new terminal start node B, http listen on 7889, p2p listen on 44002 and
connect to peer node A on port 44001:

```sh
cd $PYRSIA_HOME
cd nodeB
DEV_MODE=on RUST_LOG="pyrsia=debug,info"  ./pyrsia_node -H 0.0.0.0 -p 7889 -L /ip4/0.0.0.0/tcp/44002 --peer /ip4/127.0.0.1/tcp/44001
```

If everything goes well, you will see a line similar to the following in the logs on the terminal. (The IP address could be different than in the sample below)

```text
# DEBUG libp2p_swarm          > Connection established: PeerId("12D3KooWKzta9MMwnhA87ZKRy9PhN44X8N7twmgRhsgx1c1ZG3ex") Dialer { address: "/ip4/127.0.0.1/tcp/44001", role_override: Dialer }; Total (peer): 1. Total non-banned (peer): 1
# and in nodeA output something like:
# DEBUG libp2p_swarm            > Connection established: PeerId("12D3KooWGPwQfKN3Qvt8LosFAUxEtUUPM2BLRUqQHhFefBbJRXzY") Listener { local_addr: "/ip4/127.0.0.1/tcp/44001", send_back_addr: "/ip4/127.0.0.1/tcp/62373" }; Total (peer): 1. Total non-banned (peer): 1
```

Notice that node A and node B have now connected as peers and are able to
communicate with each other. Verify that the PeerId you see here is the same
as that for node A.

### Good news

If you saw the above lines in your logs and did not see any failure/error
messages your Pyrsia node network has now been setup. Also that means we did
not break anything. 😜

You should now be able to interact with the Pyrsia Node.

### Pyrsia CLI options

Pyrsia CLI is the `pyrsia` executable in the `$PYRSIA_HOME/target/debug` folder.
The `--help` option can show you all the ways you can interact with the network.

```sh
cd $PYRSIA_HOME/target/debug

./pyrsia --help
pyrsia_cli 0.1.1 (ed7e87160df35676815fec073be8082a8f8e9789)
Decentralized Package Network

USAGE:
    pyrsia [SUBCOMMAND]

OPTIONS:
    -h, --help       Print help information
    -V, --version    Print version information

SUBCOMMANDS:
    config -c    Pyrsia config commands
    list -l      Shows list of connected Peers
    ping         Pings configured pyrsia node
    status -s    Shows node information
    help         Print this message or the help of the given subcommand(s)
```

## Using the Pyrsia CLI

We will now look at the configuration and also configure the `pyrsia_cli` to
connect with Node B.

```sh
cd $PYRSIA_HOME
cd target/debug
./pyrsia config --show
host = 'localhost'
port = '7888'
disk_allocated = '10 GB'
```

Your Pyrsia CLI is now connected to the Pyrsia node running on port 7888.
If you would like to connect the CLI to Node B change the configuration using
the following commands:

```sh
./pyrsia config --add
Enter host:
localhost
Enter port:
7889
Enter disk space to be allocated to Pyrsia (Please enter with units ex: 10 GB):
```

If everything worked well, you will see the following success message.

```sh
Node configuration Saved !!
```

Now you are ready to use the Pyrsia CLI.

Let us run through a few examples of how you can use the Pyrsia CLI

### Get Node status

```sh
$ ./pyrsia status
Connected Peers Count:       1
Artifacts Count:             3 {"manifests": 1, "blobs": 2}
Total Disk Space Allocated:  5.84 GB
Disk Space Used:             0.0002%
```

### List all known peers

```sh
./pyrsia list
Connected Peers:
["12D3KooWH1tJB9NMuzHcEd6TU9yG4mv2Lo4J2gaXaBLpyNCrqRR9"]
```

Now that you have setup both the Pyrsia Node and Pyrsia CLI you are ready to
start using Pyrsia.

## Using Pyrsia with Docker

Once you have setup the Pyrsia nodes and the CLI you are ready to start using
Pyrsia with Docker.

## Configure Docker desktop to use node A as registry mirror

In your Docker desktop installation -> Settings -> Docker Engine where Docker
allows you to set registry-mirrors. Setup node A as a registry mirror by
adding/editing the following in the configuration.

```jsonc
 "registry-mirrors": [
   "http://192.168.0.110:7888" // (IP address of host machine: port number for node A)
 ]
```

On Mac OS X using localhost does not work(because the request is made from the
Docker Desktop VM), so you will need to specify the IP address of host machine
On Ubuntu (linuxy env) we were able to automate this and use localhost.

You will need to restart Docker Desktop. Once restarted you should be able to
pull Docker images through Pyrsia:

## Pull `alpine` docker image

First make sure Alpine is not in local Docker cache, then pull Alpine:

```sh
docker rmi alpine # remove alpine from local docker cache
docker pull alpine
```

When you pull an image of Alpine from Docker Hub, Pyrsia node A should act as
a pull-through cache and show a line similar to the following in its log:

```sh
# DEBUG pyrsia::docker::v2::handlers::blobs> Step 3: "sha256:3d243047344378e9b7136d552d48feb7ea8b6fe14ce0990e0cc011d5e369626a" successfully stored locally from docker.io
```

You can try the same with node B acting as the registry mirror.

Change the docker registry mirror to node B

```jsonc
 "registry-mirrors": [
   "http://192.168.0.110:7889" // (IP address of host machine: port number of node B)
 ]
```

Remove alpine and perform a docker pull again

```sh
docker rmi alpine # remove alpine from local docker cache
docker pull alpine
```

Now node B is acting as the pull-through cache and should show a line similar
to the following in its log, indicating `alpine` was retrieved from the
Pyrsia network (in this case node A).

```sh
# DEBUG pyrsia::docker::v2::handlers::blobs> Step 2: "sha256:3d243047344378e9b7136d552d48feb7ea8b6fe14ce0990e0cc011d5e369626a" successfully stored locally from Pyrsia network.
```

Success!!!

You have just built yourself a working Pyrsia network. Enjoy using it and
showcasing it to your teams and please share any feedback!

Next you can follow the [demo instructions](https://pyrsia.io/docs/tutorials/demo/) and setup a real Pyrsia network and use it with your CI system.
