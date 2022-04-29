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

use crate::network::artifact_protocol::ArtifactResponse;
use crate::network::client::{ArtifactHash, ArtifactType};
use futures::channel::oneshot;
use libp2p::core::{Multiaddr, PeerId};
use libp2p::request_response::ResponseChannel;
use std::collections::HashSet;
use std::error::Error;
use std::fmt::{Display, Formatter};

/// Commands are sent by the [`Client`] to the [`PyrsiaEventLoop`].
/// Each command matches exactly with one if the functions that are
/// defined in `Client`.
#[derive(Debug)]
pub enum Command {
    Listen {
        addr: Multiaddr,
        sender: oneshot::Sender<Result<(), Box<dyn Error + Send>>>,
    },
    Dial {
        peer_addr: Multiaddr,
        sender: oneshot::Sender<Result<(), Box<dyn Error + Send>>>,
    },
    ListPeers {
        peer_id: PeerId,
        sender: oneshot::Sender<HashSet<PeerId>>,
    },
    Provide {
        artifact_type: ArtifactType,
        artifact_hash: ArtifactHash,
        sender: oneshot::Sender<()>,
    },
    ListProviders {
        artifact_type: ArtifactType,
        artifact_hash: ArtifactHash,
        sender: oneshot::Sender<HashSet<PeerId>>,
    },
    RequestArtifact {
        artifact_type: ArtifactType,
        artifact_hash: ArtifactHash,
        peer: PeerId,
        sender: oneshot::Sender<Result<Vec<u8>, Box<dyn Error + Send>>>,
    },
    RespondArtifact {
        artifact: Vec<u8>,
        channel: ResponseChannel<ArtifactResponse>,
    },
}

impl Display for Command {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let name = match self {
            Command::Listen { .. } => "Listen",
            Command::Dial { .. } => "Dial",
            Command::ListPeers { .. } => "ListPeers",
            Command::Provide { .. } => "Provide",
            Command::ListProviders { .. } => "ListProviders",
            Command::RequestArtifact { .. } => "RequestArtifact",
            Command::RespondArtifact { .. } => "RespondArtifact",
        };
        write!(f, "{}", name)
    }
}
