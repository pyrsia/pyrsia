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
use std::str::FromStr;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_LISTEN_ADDRESS: &str = "/ip4/0.0.0.0/tcp/0";
const DEFAULT_PORT: &str = "7888";

/// Application to connect to and participate in the Pyrsia network
#[derive(Debug, Parser)]
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
    /// The mode (client-listen, client-dial) for dcutr or NoRelay
    #[clap(long)]
    pub mode: Mode,
    /// The listening address
    #[clap(long)]
    pub relay_address: Option<Multiaddr>,
}

#[derive(Debug, Parser, PartialEq)]
pub enum Mode {
    Dial,
    Listen,
    NoRelay,
}

impl FromStr for Mode {
    type Err = String;
    fn from_str(mode: &str) -> Result<Self, Self::Err> {
        match mode {
            "dial" => Ok(Mode::Dial),
            "listen" => Ok(Mode::Listen),
            "NoRelay" => Ok(Mode::NoRelay),
            _ => Err("Expected either 'dial' or 'listen'".to_string()),
        }
    }
}
