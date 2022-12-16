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

use crate::network::artifact_protocol::{ArtifactExchangeCodec, ArtifactRequest, ArtifactResponse};
use crate::network::blockchain_protocol::{
    BlockchainExchangeCodec, BlockchainRequest, BlockchainResponse,
};
use crate::network::idle_metric_protocol::{
    IdleMetricExchangeCodec, IdleMetricRequest, IdleMetricResponse,
};

use crate::network::build_protocol::{BuildExchangeCodec, BuildRequest, BuildResponse};
use crate::network::build_status_protocol::{
    BuildStatusExchangeCodec, BuildStatusRequest, BuildStatusResponse,
};
use libp2p::autonat;
use libp2p::gossipsub;
use libp2p::identify;
use libp2p::kad::record::store::MemoryStore;
use libp2p::kad::{Kademlia, KademliaEvent};
use libp2p::request_response::{RequestResponse, RequestResponseEvent};
use libp2p::swarm::NetworkBehaviour;

/// Defines the [`NetworkBehaviour`] to be used in the libp2p
/// Swarm. The PyrsiaNetworkBehaviour consists of the following
/// behaviours:
///
/// * [`Identify`]
/// * [`Kademlia`]
/// * [`RequestResponse`] for exchanging artifacts, idle metrics and
/// blockchain updates
#[derive(NetworkBehaviour)]
#[behaviour(out_event = "PyrsiaNetworkEvent")]
pub struct PyrsiaNetworkBehaviour {
    pub auto_nat: autonat::Behaviour,
    pub gossipsub: gossipsub::Gossipsub,
    pub identify: identify::Behaviour,
    pub kademlia: Kademlia<MemoryStore>,
    pub request_response: RequestResponse<ArtifactExchangeCodec>,
    pub build_request_response: RequestResponse<BuildExchangeCodec>,
    pub idle_metric_request_response: RequestResponse<IdleMetricExchangeCodec>,
    pub blockchain_request_response: RequestResponse<BlockchainExchangeCodec>,
    pub build_status_request_response: RequestResponse<BuildStatusExchangeCodec>,
}

/// Each event in the `PyrsiaNetworkBehaviour` is wrapped in a
/// `PyrsiaNetworkEvent`.
#[derive(Debug)]
pub enum PyrsiaNetworkEvent {
    AutoNat(autonat::Event),
    Gossipsub(gossipsub::GossipsubEvent),
    Identify(Box<identify::Event>),
    Kademlia(Box<KademliaEvent>),
    RequestResponse(RequestResponseEvent<ArtifactRequest, ArtifactResponse>),
    BuildRequestResponse(RequestResponseEvent<BuildRequest, BuildResponse>),
    IdleMetricRequestResponse(RequestResponseEvent<IdleMetricRequest, IdleMetricResponse>),
    BlockchainRequestResponse(RequestResponseEvent<BlockchainRequest, BlockchainResponse>),
    BuildStatusRequestResponse(RequestResponseEvent<BuildStatusRequest, BuildStatusResponse>),
}

impl From<autonat::Event> for PyrsiaNetworkEvent {
    fn from(event: autonat::Event) -> Self {
        PyrsiaNetworkEvent::AutoNat(event)
    }
}

impl From<gossipsub::GossipsubEvent> for PyrsiaNetworkEvent {
    fn from(event: gossipsub::GossipsubEvent) -> Self {
        PyrsiaNetworkEvent::Gossipsub(event)
    }
}

impl From<identify::Event> for PyrsiaNetworkEvent {
    fn from(event: identify::Event) -> Self {
        PyrsiaNetworkEvent::Identify(Box::new(event))
    }
}

impl From<KademliaEvent> for PyrsiaNetworkEvent {
    fn from(event: KademliaEvent) -> Self {
        PyrsiaNetworkEvent::Kademlia(Box::new(event))
    }
}

impl From<RequestResponseEvent<ArtifactRequest, ArtifactResponse>> for PyrsiaNetworkEvent {
    fn from(event: RequestResponseEvent<ArtifactRequest, ArtifactResponse>) -> Self {
        PyrsiaNetworkEvent::RequestResponse(event)
    }
}

impl From<RequestResponseEvent<BuildRequest, BuildResponse>> for PyrsiaNetworkEvent {
    fn from(event: RequestResponseEvent<BuildRequest, BuildResponse>) -> Self {
        PyrsiaNetworkEvent::BuildRequestResponse(event)
    }
}

impl From<RequestResponseEvent<IdleMetricRequest, IdleMetricResponse>> for PyrsiaNetworkEvent {
    fn from(event: RequestResponseEvent<IdleMetricRequest, IdleMetricResponse>) -> Self {
        PyrsiaNetworkEvent::IdleMetricRequestResponse(event)
    }
}

impl From<RequestResponseEvent<BlockchainRequest, BlockchainResponse>> for PyrsiaNetworkEvent {
    fn from(event: RequestResponseEvent<BlockchainRequest, BlockchainResponse>) -> Self {
        PyrsiaNetworkEvent::BlockchainRequestResponse(event)
    }
}

impl From<RequestResponseEvent<BuildStatusRequest, BuildStatusResponse>> for PyrsiaNetworkEvent {
    fn from(event: RequestResponseEvent<BuildStatusRequest, BuildStatusResponse>) -> Self {
        PyrsiaNetworkEvent::BuildStatusRequestResponse(event)
    }
}
