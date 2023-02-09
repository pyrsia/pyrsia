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

use crate::artifact_service::model::PackageType;
use crate::network::artifact_protocol::ArtifactResponse;
use crate::network::blockchain_protocol::BlockchainResponse;
use crate::network::build_protocol::BuildResponse;
use crate::network::build_status_protocol::BuildStatusResponse;
use crate::network::idle_metric_protocol::{IdleMetricResponse, PeerMetrics};
use crate::node_api::model::request::Status;
use libp2p::core::{Multiaddr, PeerId};
use libp2p::gossipsub;
use libp2p::request_response::ResponseChannel;
use std::collections::HashSet;
use strum_macros::Display;
use tokio::sync::oneshot;

/// Commands are sent by the [`Client`] to the [`PyrsiaEventLoop`].
/// Each command matches exactly with one if the functions that are
/// defined in `Client`.
#[derive(Debug, Display)]
pub enum Command {
    AddProbe {
        peer_id: PeerId,
        probe_addr: Multiaddr,
        sender: oneshot::Sender<anyhow::Result<()>>,
    },
    BroadcastBlock {
        topic: gossipsub::IdentTopic,
        block: Vec<u8>,
        sender: oneshot::Sender<anyhow::Result<()>>,
    },
    BootstrapDht {
        sender: oneshot::Sender<anyhow::Result<()>>,
    },
    Listen {
        addr: Multiaddr,
        sender: oneshot::Sender<anyhow::Result<()>>,
    },
    Dial {
        peer_id: PeerId,
        peer_addr: Multiaddr,
        sender: oneshot::Sender<anyhow::Result<()>>,
    },
    ListPeers {
        sender: oneshot::Sender<HashSet<PeerId>>,
    },
    Status {
        sender: oneshot::Sender<Status>,
    },
    Provide {
        artifact_id: String,
        sender: oneshot::Sender<()>,
    },
    ListProviders {
        artifact_id: String,
        sender: oneshot::Sender<HashSet<PeerId>>,
    },
    RequestBuild {
        peer: PeerId,
        package_type: PackageType,
        package_specific_id: String,
        sender: oneshot::Sender<anyhow::Result<String>>,
    },
    RespondBuild {
        build_id: String,
        channel: ResponseChannel<BuildResponse>,
    },
    RequestArtifact {
        artifact_id: String,
        peer: PeerId,
        sender: oneshot::Sender<anyhow::Result<Vec<u8>>>,
    },
    RespondArtifact {
        artifact: Vec<u8>,
        channel: ResponseChannel<ArtifactResponse>,
    },
    RequestIdleMetric {
        peer: PeerId,
        sender: oneshot::Sender<anyhow::Result<PeerMetrics>>,
    },
    RespondIdleMetric {
        metric: PeerMetrics,
        channel: ResponseChannel<IdleMetricResponse>,
    },
    RequestBlockchain {
        data: Vec<u8>,
        peer: PeerId,
        sender: oneshot::Sender<anyhow::Result<Vec<u8>>>,
    },
    RespondBlockchain {
        data: Vec<u8>,
        channel: ResponseChannel<BlockchainResponse>,
    },
    RequestBuildStatus {
        peer: PeerId,
        build_id: String,
        sender: oneshot::Sender<anyhow::Result<String>>,
    },
    RespondBuildStatus {
        status: String,
        channel: ResponseChannel<BuildStatusResponse>,
    },
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;

    #[test]
    fn command_correctly_implements_display() {
        let (sender, _) = oneshot::channel();
        let addr: Multiaddr = "/ip4/127.0.0.1".parse().unwrap();

        assert_eq!(
            String::from("Listen"),
            Command::Listen { addr, sender }.to_string()
        );
    }
}
