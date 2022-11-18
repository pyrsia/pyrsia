/*
   Copyright 2021 JFrog Ltd

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

use clap::Parser;
use libp2p::Multiaddr;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_LISTEN_ADDRESS: &str = "/ip4/0.0.0.0/tcp/0";
const DEFAULT_MAX_PROVIDED_KEYS: &str = "32768";
const DEFAULT_MAPPING_SERVICE_ENDPOINT: &str =
    "https://raw.githubusercontent.com/pyrsia/pyrsia-mappings/main/";
const DEFAULT_PIPELINE_SERVICE_ENDPOINT: &str = "http://localhost:8080";
const DEFAULT_PORT: &str = "7888";
const DEFAULT_BOOTSTRAP_URL: &str = "http://boot.pyrsia.link/status";

/// Application to connect to and participate in the Pyrsia network
#[derive(Clone, Debug, Parser)]
#[clap(name = "Pyrsia Node")]
pub struct PyrsiaNodeArgs {
    /// The host address to bind to for the Docker API
    #[clap(long, short = 'H', default_value = DEFAULT_HOST)]
    pub host: String,
    /// The address to listen to for incoming requests from other pyrsia nodes
    #[clap(long = "listen", short = 'L', default_value = DEFAULT_LISTEN_ADDRESS)]
    pub listen_address: Multiaddr,
    /// the port to listen to for the Docker API
    #[clap(long, short, default_value = DEFAULT_PORT)]
    pub port: String,
    /// An address to connect with another Pyrsia Node (eg /ip4/127.0.0.1/tcp/45153/p2p/12D3KooWKsHbKbcVgyiRRgeXGCK4bp3MngnSU7ioeKTfQzd18B2v)
    #[clap(long, short = 'P')]
    pub peer: Option<Multiaddr>,
    /// Initialization mode, used only for the first authorized node in the Pyrsia network to initialize the Pyrsia network
    #[clap(long)]
    pub init_blockchain: bool,
    /// An address to use for probing AutoNAT connections
    #[clap(long, short = 'R')]
    pub probe: Option<Multiaddr>,
    /// listen_only mode - don't try to connect to any peers at startup
    #[clap(long)]
    pub listen_only: bool,
    #[clap(long, short = 'B', default_value = DEFAULT_BOOTSTRAP_URL)]
    pub bootstrap_url: String,
    /// The maximum number of keys that can be provided on the network by this Pyrsia Node.
    #[clap(long, default_value = DEFAULT_MAX_PROVIDED_KEYS)]
    pub max_provided_keys: usize,
    /// The http endpoint where the mapping service will fetch mapping info from.
    #[clap(long, default_value = DEFAULT_MAPPING_SERVICE_ENDPOINT)]
    pub mapping_service_endpoint: String,
    /// The http endpoint of the external build pipeline that the pipeline service will use to communicate with.
    #[clap(long, default_value = DEFAULT_PIPELINE_SERVICE_ENDPOINT)]
    pub pipeline_service_endpoint: String,
}
