# How to Install Pyrsia and test it?


Pyrsia consists of 2 components, they are: 
 - CLI
 - Nodes
 
 Pyrsia CLI is the utility tool that allows you to pull artifacts from various registries. The Pyrsia CLI follows simple syntax, you can easily pull an artifact ,
 configure pyrsia nodes and see the essential stats like how many nodes are connected to the network, etc.
 
 example:
 ```bash
    ./pyrsia.sh config -s
 ```
 
 #### DISCLAIMER: the process of pulling an artifact directly is still under development
 
 
 Once you have configured Docker Daemon to accept localhost:7888, you can pull docker images through pyrsia node instead.
 
 [Reference to configuring](https://github.com/pyrsia/pyrsia/tree/main/pyrsia_node)
 
 
 The pyrsia node determines how  the artifacts(packages, Images) are being pulled, and where thet are pulled from.
 
 
 ## To build the CLI:
1. Clone the repository
2. Change directories to `pyrsia_cli`
3. Run `cargo build`
4. Go to target/debug
5. Add `pyrsia` CLI to the Path
6. Run Pyrsia commands


## To Build the node:
1. Change directories to node
2. Build
