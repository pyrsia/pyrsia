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

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
pub mod tests {
    use crate::artifact_service::service::ArtifactService;
    use crate::blockchain_service::event::{BlockchainEvent, BlockchainEventClient};
    use crate::build_service::event::{BuildEvent, BuildEventClient};
    use crate::network::client::command::Command;
    use crate::network::client::Client;
    use crate::transparency_log::log::TransparencyLogService;
    use crate::verification_service::service::VerificationService;
    use libp2p::gossipsub::IdentTopic;
    use libp2p::identity::Keypair;
    use std::env;
    use std::fs;
    use std::path;
    use tokio::sync::mpsc::{self, Receiver};

    pub fn create_blockchain_event_client() -> (BlockchainEventClient, Receiver<BlockchainEvent>) {
        let (sender, receiver) = mpsc::channel(1);
        (BlockchainEventClient::new(sender), receiver)
    }

    pub fn create_build_event_client() -> (BuildEventClient, Receiver<BuildEvent>) {
        let (sender, receiver) = mpsc::channel(1);
        (BuildEventClient::new(sender), receiver)
    }

    pub fn create_p2p_client() -> (Client, Receiver<Command>) {
        let (sender, receiver) = mpsc::channel(1);
        (
            Client::new(
                sender,
                Keypair::generate_ed25519().public().to_peer_id(),
                IdentTopic::new("pyrsia-topic"),
            ),
            receiver,
        )
    }

    pub fn create_artifact_service<P: AsRef<path::Path>>(
        artifact_path: P,
    ) -> (
        ArtifactService,
        Receiver<BlockchainEvent>,
        Receiver<BuildEvent>,
        Receiver<Command>,
    ) {
        let (blockchain_event_client, blockchain_event_receiver) = create_blockchain_event_client();
        let (build_event_client, build_event_receiver) = create_build_event_client();
        let (p2p_client, p2p_command_receiver) = create_p2p_client();

        (
            ArtifactService::new(
                &artifact_path,
                blockchain_event_client,
                build_event_client,
                p2p_client,
            )
            .unwrap(),
            blockchain_event_receiver,
            build_event_receiver,
            p2p_command_receiver,
        )
    }

    pub fn create_artifact_service_with_p2p_client<P: AsRef<path::Path>>(
        artifact_path: P,
        p2p_client: Client,
    ) -> (
        ArtifactService,
        Receiver<BlockchainEvent>,
        Receiver<BuildEvent>,
    ) {
        let (blockchain_event_client, blockchain_event_receiver) = create_blockchain_event_client();
        let (build_event_client, build_event_receiver) = create_build_event_client();

        (
            ArtifactService::new(
                &artifact_path,
                blockchain_event_client,
                build_event_client,
                p2p_client,
            )
            .unwrap(),
            blockchain_event_receiver,
            build_event_receiver,
        )
    }

    pub fn create_transparency_log_service<P: AsRef<path::Path>>(
        repository_path: P,
    ) -> (TransparencyLogService, Receiver<BlockchainEvent>) {
        let (blockchain_event_client, blockchain_event_receiver) = create_blockchain_event_client();

        (
            TransparencyLogService::new(&repository_path, blockchain_event_client).unwrap(),
            blockchain_event_receiver,
        )
    }

    pub fn create_verification_service() -> (VerificationService, Receiver<BuildEvent>) {
        let (build_event_client, build_event_receiver) = create_build_event_client();

        (
            VerificationService::new(build_event_client).unwrap(),
            build_event_receiver,
        )
    }

    pub fn setup() -> path::PathBuf {
        let tmp_dir = tempfile::tempdir()
            .expect("could not create temporary directory")
            .into_path();

        env::set_var("PYRSIA_ARTIFACT_PATH", tmp_dir.to_str().unwrap());
        env::set_var("DEV_MODE", "on");

        tmp_dir
    }

    pub fn teardown(tmp_dir: path::PathBuf) {
        if tmp_dir.exists() {
            fs::remove_dir_all(&tmp_dir)
                .unwrap_or_else(|_| panic!("unable to remove test directory {:?}", tmp_dir));
        }

        env::remove_var("PYRSIA_ARTIFACT_PATH");
        env::remove_var("DEV_MODE");
    }
}
