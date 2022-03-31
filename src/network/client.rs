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

pub mod command;

use crate::network::artifact_protocol::ArtifactResponse;
use crate::network::client::command::Command;
use futures::channel::{mpsc, oneshot};
use futures::prelude::*;
use libp2p::core::{Multiaddr, PeerId};
use libp2p::request_response::ResponseChannel;
use log::debug;
use std::collections::HashSet;
use std::error;

#[derive(Clone)]
pub struct Client {
    pub sender: mpsc::Sender<Command>,
    pub local_peer_id: PeerId,
}

impl Client {
    /// Instruct the p2p swarm to start listening on the specified address.
    pub async fn listen(&mut self, addr: &Multiaddr) -> Result<(), Box<dyn error::Error + Send>> {
        debug!("p2p::Client::listen {:?}", addr);

        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::Listen {
                addr: addr.clone(),
                sender,
            })
            .await
            .expect("Command receiver not to be dropped.");
        receiver.await.expect("Sender not to be dropped.")
    }

    /// Dial a peer with the specified address.
    pub async fn dial(
        &mut self,
        peer_addr: &Multiaddr,
    ) -> Result<(), Box<dyn error::Error + Send>> {
        debug!("p2p::Client::dial {:?}", peer_addr);

        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::Dial {
                peer_addr: peer_addr.clone(),
                sender,
            })
            .await
            .expect("Command receiver not to be dropped.");
        receiver.await.expect("Sender not to be dropped.")
    }

    /// List the peers that this node is connected to.
    pub async fn list_peers(&mut self) -> HashSet<PeerId> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::ListPeers {
                peer_id: self.local_peer_id,
                sender,
            })
            .await
            .expect("Command receiver not to be dropped.");
        receiver.await.expect("Sender not to be dropped.")
    }

    /// Inform the p2p network that this node is currently a
    /// provider of the artifact with the specified `hash`.
    pub async fn provide(&mut self, hash: &str) {
        debug!("p2p::Client::provide {:?}", hash);

        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::Provide {
                hash: String::from(hash),
                sender,
            })
            .await
            .expect("Command receiver not to be dropped.");
        receiver.await.expect("Sender not to be dropped.")
    }

    /// List all peers in the p2p network that are providing
    /// the artifact with the specified `hash`.
    pub async fn list_providers(&mut self, hash: String) -> HashSet<PeerId> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::ListProviders { hash, sender })
            .await
            .expect("Command receiver not to be dropped.");
        receiver.await.expect("Sender not to be dropped.")
    }

    /// Request an artifact with the specified `hash` from the
    /// p2p network.
    pub async fn request_artifact(
        &mut self,
        peer: &PeerId,
        hash: String,
    ) -> Result<Vec<u8>, Box<dyn error::Error + Send>> {
        debug!("p2p::Client::request_artifact {:?}: {:?}", peer, hash);

        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::RequestArtifact {
                hash,
                peer: *peer,
                sender,
            })
            .await
            .expect("Command receiver not to be dropped.");
        receiver.await.expect("Sender not to be dropped.")
    }

    /// Put the artifact as a response to an incoming request.
    pub async fn respond_artifact(
        &mut self,
        artifact: Vec<u8>,
        channel: ResponseChannel<ArtifactResponse>,
    ) {
        debug!("p2p::Client::respond_artifact size={:?}", artifact.len());

        self.sender
            .send(Command::RespondArtifact { artifact, channel })
            .await
            .expect("Command receiver not to be dropped.");
    }
}
