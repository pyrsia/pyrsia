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

use crate::artifacts_repository::hash_util::HashAlgorithm;
use crate::network::p2p;
use crate::node_manager::handlers::get_artifact;
use libp2p::request_response::ResponseChannel;
use libp2p::Multiaddr;
use log::info;

/// Reach out to another node with the specified address
pub async fn dial_other_peer(mut p2p_client: p2p::Client, to_dial: Multiaddr) {
    p2p_client
        .dial(to_dial.clone())
        .await
        .expect("Dial to succeed.");
    info!("Dialed {:?}", to_dial);
}

/// Respond to a RequestArtifact event by getting the artifact from
/// the ArtifactManager.
pub async fn handle_request_artifact(
    mut p2p_client: p2p::Client,
    hash: &str,
    channel: ResponseChannel<p2p::ArtifactResponse>,
) {
    let decoded_hash = hex::decode(&hash.get(7..).unwrap()).unwrap();
    match get_artifact(&decoded_hash, HashAlgorithm::SHA256) {
        Ok(content) => p2p_client.respond_artifact(content, channel).await,
        Err(e) => info!(
            "This node does not provide artifact {}. Error: {:?}",
            hash, e
        ),
    }
}
