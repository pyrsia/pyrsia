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

use super::model::PackageType;
use super::storage::ArtifactStorage;
use crate::build_service::error::BuildError;
use crate::build_service::event::BuildEventClient;
use crate::build_service::model::BuildResult;
use crate::network::client::Client;
use crate::transparency_log::log::{
    AddArtifactRequest, TransparencyLog, TransparencyLogError, TransparencyLogService,
};
use anyhow::{bail, Context};
use itertools::Itertools;
use libp2p::PeerId;
use log::{debug, info, warn};
use multihash::Hasher;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::str;
use std::str::FromStr;

/// The artifact service is the component that handles everything related to
/// pyrsia artifacts. It allows artifacts to be retrieved and added to the
/// pyrsia network by requesting a build from source.
#[derive(Clone)]
pub struct ArtifactService {
    pub artifact_storage: ArtifactStorage,
    build_event_client: BuildEventClient,
    pub transparency_log_service: TransparencyLogService,
    pub p2p_client: Client,
}

impl ArtifactService {
    pub fn new<P: AsRef<Path>>(
        artifact_path: P,
        transparency_log_service: TransparencyLogService,
        build_event_client: BuildEventClient,
        p2p_client: Client,
    ) -> anyhow::Result<Self> {
        let artifact_storage = ArtifactStorage::new(&artifact_path)?;
        Ok(ArtifactService {
            artifact_storage,
            build_event_client,
            transparency_log_service,
            p2p_client,
        })
    }

    pub async fn request_build(
        &self,
        package_type: PackageType,
        package_specific_id: String,
    ) -> Result<String, BuildError> {
        debug!(
            "Request build of {:?} {:?}",
            package_type, package_specific_id
        );

        let local_peer_id = self.p2p_client.local_peer_id;
        debug!("Got local node with peer_id: {:?}", local_peer_id.clone());

        let nodes = self
            .transparency_log_service
            .get_authorized_nodes()
            .map_err(|e| BuildError::InitializationFailed(e.to_string()))?;

        if nodes.is_empty() {
            warn!("No authorized nodes found");
            return Err(BuildError::InitializationFailed(String::from(
                "No authorized nodes found",
            )));
        }

        let peer_id = match nodes
            .iter()
            .map(|node| PeerId::from_str(&node.node_id).unwrap())
            .find_or_last(|&auth_peer_id| local_peer_id.eq(&auth_peer_id))
        {
            Some(auth_peer_id) => {
                debug!(
                    "Got authorized node with peer_id: {:?}",
                    auth_peer_id.clone()
                );
                auth_peer_id
            }
            None => panic!("Error unexpected looking for authorized nodes"),
        };

        if local_peer_id.eq(&peer_id) {
            debug!("Start local build in authorized node");
            self.build_event_client
                .start_build(package_type, package_specific_id)
                .await
        } else {
            debug!("Request build in authorized node from p2p network");
            self.p2p_client
                .clone()
                .request_build(&peer_id, package_type, package_specific_id.clone())
                .await
                .map_err(|e| BuildError::InitializationFailed(e.to_string()))
        }
    }

    pub async fn handle_build_result(
        &mut self,
        build_id: &str,
        build_result: BuildResult,
    ) -> Result<(), anyhow::Error> {
        let package_specific_id = build_result.package_specific_id.as_str();

        info!(
            "Build with ID {} completed successfully for package type {:?} and package specific ID {}",
            build_id, build_result.package_type, package_specific_id
        );

        for artifact in build_result.artifacts.iter() {
            let add_artifact_request = AddArtifactRequest {
                package_type: build_result.package_type,
                package_specific_id: package_specific_id.to_owned(),
                num_artifacts: build_result.artifacts.len() as u32,
                package_specific_artifact_id: artifact.artifact_specific_id.clone(),
                artifact_hash: artifact.artifact_hash.clone(),
            };

            info!(
                "Adding artifact to transparency log: {:?}",
                add_artifact_request
            );

            let add_artifact_transparency_log = self
                .transparency_log_service
                .add_artifact(add_artifact_request)
                .await?;
            info!(
                "Transparency Log for build with ID {} successfully created.",
                build_id
            );

            self.transparency_log_service
                .write_transparency_log(&add_artifact_transparency_log)?;

            self.put_artifact_from_build_result(
                &artifact.artifact_location,
                &add_artifact_transparency_log.artifact_id,
            )
            .await?;

            self.p2p_client
                .provide(&add_artifact_transparency_log.artifact_id)
                .await?;
        }

        Ok(())
    }

    pub async fn handle_block_added(
        &mut self,
        payloads: Vec<Vec<u8>>,
    ) -> Result<(), anyhow::Error> {
        if payloads.len() == 1 {
            let transparency_log: TransparencyLog = serde_json::from_slice(&payloads[0])?;
            if let Err(TransparencyLogError::LogNotFound { .. }) = self
                .transparency_log_service
                .find_transparency_log(&transparency_log.id)
            {
                self.transparency_log_service
                    .write_transparency_log(&transparency_log)?;
            }
        }

        Ok(())
    }

    async fn put_artifact_from_build_result(
        &self,
        artifact_location: &Path,
        artifact_id: &str,
    ) -> Result<(), anyhow::Error> {
        let artifact_file = File::open(artifact_location)?;
        let mut artifact_reader = BufReader::new(artifact_file);
        self.put_artifact(artifact_id, &mut artifact_reader)
    }

    /// Given artifact_id & reader, push artifact to artifact_storage
    fn put_artifact(&self, artifact_id: &str, reader: &mut impl Read) -> Result<(), anyhow::Error> {
        info!("put_artifact with id: {}", artifact_id);
        self.artifact_storage
            .push_artifact(reader, artifact_id)
            .context("Error from put_artifact")
    }

    /// Retrieve the artifact data for the specified package. If the artifact
    /// is not available locally, the service will try to fetch the artifact
    /// from the p2p network.
    pub async fn get_artifact(
        &mut self,
        package_type: PackageType,
        package_specific_artifact_id: &str,
    ) -> anyhow::Result<Vec<u8>> {
        let transparency_log = self
            .transparency_log_service
            .get_artifact(&package_type, package_specific_artifact_id)?;

        let artifact = match self
            .get_artifact_locally(&transparency_log.artifact_id)
            .await
        {
            Ok(artifact) => Ok(artifact),
            Err(_) => {
                self.get_artifact_from_peers(&transparency_log.artifact_id)
                    .await
            }
        }?;

        self.verify_artifact(&transparency_log, &artifact).await?;

        Ok(artifact)
    }

    /// Retrieve the artifact data specified by `artifact_id` from the local storage.
    pub async fn get_artifact_locally(
        &mut self,
        artifact_id: &str,
    ) -> Result<Vec<u8>, anyhow::Error> {
        let artifact = self.artifact_storage.pull_artifact(artifact_id)?;
        let mut buf_reader = BufReader::new(artifact);
        let mut blob_content = Vec::new();
        buf_reader.read_to_end(&mut blob_content)?;
        Ok(blob_content)
    }

    /// Retrieve the artifact logs for the specified package.
    pub async fn get_logs_for_artifact(
        &mut self,
        package_type: PackageType,
        package_specific_id: &str,
    ) -> anyhow::Result<Vec<TransparencyLog>> {
        let transparency_logs = self
            .transparency_log_service
            .search_transparency_logs(&package_type, package_specific_id)?;

        Ok(transparency_logs)
    }

    async fn get_artifact_from_peers(
        &mut self,
        artifact_id: &str,
    ) -> Result<Vec<u8>, anyhow::Error> {
        let providers = self.p2p_client.list_providers(artifact_id).await?;

        match self.p2p_client.get_idle_peer(providers).await? {
            Some(peer_id) => self.get_artifact_from_peer(&peer_id, artifact_id).await,
            None => {
                bail!(
                    "Artifact with id {} is not available on the p2p network.",
                    artifact_id
                )
            }
        }
    }

    async fn get_artifact_from_peer(
        &mut self,
        peer_id: &PeerId,
        artifact_id: &str,
    ) -> Result<Vec<u8>, anyhow::Error> {
        let artifact = self
            .p2p_client
            .request_artifact(peer_id, artifact_id)
            .await?;

        let mut buf_reader = BufReader::new(artifact.as_slice());

        self.put_artifact(artifact_id, &mut buf_reader)?;
        self.get_artifact_locally(artifact_id).await
    }

    async fn verify_artifact(
        &mut self,
        transparency_log: &TransparencyLog,
        artifact: &[u8],
    ) -> Result<(), TransparencyLogError> {
        let mut sha256 = multihash::Sha2_256::default();
        sha256.update(artifact);
        let calculated_hash = hex::encode(sha256.finalize());

        if transparency_log.artifact_hash == calculated_hash {
            Ok(())
        } else {
            Err(TransparencyLogError::InvalidHash {
                id: transparency_log.package_specific_artifact_id.clone(),
                invalid_hash: calculated_hash,
                actual_hash: transparency_log.artifact_hash.clone(),
            })
        }
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use crate::blockchain_service::service::BlockchainService;
    use crate::build_service::event::BuildEvent;
    use crate::network::client::command::Command;
    use crate::network::idle_metric_protocol::PeerMetrics;
    use crate::util::test_util;
    use libp2p::identity::ed25519::Keypair;
    use libp2p::identity::PublicKey;
    use sha2::{Digest, Sha256};
    use std::collections::HashSet;
    use std::env;
    use std::path::PathBuf;
    use std::sync::Arc;
    use tokio::sync::{mpsc, Mutex};
    use tokio::task;

    const VALID_ARTIFACT_HASH: [u8; 32] = [
        0x86, 0x5c, 0x8d, 0x98, 0x8b, 0xe4, 0x66, 0x9f, 0x3e, 0x48, 0xf7, 0x3b, 0x98, 0xf9, 0xbc,
        0x25, 0x7, 0xbe, 0x2, 0x46, 0xea, 0x35, 0xe0, 0x9, 0x8c, 0xf6, 0x5, 0x4d, 0x36, 0x44, 0xc1,
        0x4f,
    ];

    fn create_p2p_client(keypair: &Keypair) -> (mpsc::Receiver<Command>, Client) {
        let (command_sender, command_receiver) = mpsc::channel(1);
        let p2p_client = Client {
            sender: command_sender,
            local_peer_id: PublicKey::Ed25519(keypair.public()).to_peer_id(),
        };

        (command_receiver, p2p_client)
    }

    async fn create_artifact_service(
        artifact_path: impl AsRef<Path>,
        keypair: &Keypair,
        p2p_client: Client,
    ) -> (mpsc::Receiver<BuildEvent>, ArtifactService) {
        let blockchain_service = BlockchainService::init_first_blockchain_node(
            keypair,
            keypair,
            p2p_client.clone(),
            &artifact_path,
        )
        .await
        .expect("Creating BlockchainService failed");

        let transparency_log_service =
            TransparencyLogService::new(&artifact_path, Arc::new(Mutex::new(blockchain_service)))
                .expect("Creating TransparencyLogService failed");

        let (build_event_sender, build_event_receiver) = mpsc::channel(1);
        let build_event_client = BuildEventClient::new(build_event_sender);

        let artifact_service = ArtifactService::new(
            &artifact_path,
            transparency_log_service,
            build_event_client,
            p2p_client,
        )
        .expect("Creating ArtifactService failed");

        (build_event_receiver, artifact_service)
    }

    #[tokio::test]
    async fn test_put_and_get_artifact() {
        let tmp_dir = test_util::tests::setup();

        let keypair = Keypair::generate();

        let (mut command_receiver, p2p_client) = create_p2p_client(&keypair);
        let (_build_event_receiver, mut artifact_service) =
            create_artifact_service(&tmp_dir, &keypair, p2p_client.clone()).await;

        tokio::spawn(async move {
            loop {
                match command_receiver.recv().await {
                    Some(Command::ListPeers { sender, .. }) => {
                        let _ = sender.send(HashSet::new());
                    }
                    _ => panic!("Command must match Command::ListPeers"),
                }
            }
        });

        let package_type = PackageType::Docker;
        let package_specific_id = "package_specific_id";
        let package_specific_artifact_id = "package_specific_artifact_id";
        let transparency_log = artifact_service
            .transparency_log_service
            .add_artifact(AddArtifactRequest {
                package_type,
                package_specific_id: package_specific_id.to_owned(),
                num_artifacts: 8,
                package_specific_artifact_id: package_specific_artifact_id.to_owned(),
                artifact_hash: hex::encode(VALID_ARTIFACT_HASH),
            })
            .await
            .unwrap();
        artifact_service
            .transparency_log_service
            .write_transparency_log(&transparency_log)
            .unwrap();

        //put the artifact
        artifact_service
            .put_artifact(
                &transparency_log.artifact_id,
                &mut get_file_reader().unwrap(),
            )
            .context("Error from put_artifact")
            .unwrap();

        // pull artifact
        let future = {
            artifact_service
                .get_artifact(package_type, package_specific_artifact_id)
                .await
                .context("Error from get_artifact")
        };
        let file = task::spawn_blocking(|| future).await.unwrap().unwrap();

        //validate pulled artifact with the actual data
        let mut s = String::new();
        get_file_reader().unwrap().read_to_string(&mut s).unwrap();

        let s1 = match str::from_utf8(file.as_slice()) {
            Ok(v) => v,
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        };
        assert_eq!(s, s1);

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_from_peers() {
        let tmp_dir = test_util::tests::setup();

        let keypair = Keypair::generate();

        let (mut command_receiver, p2p_client) = create_p2p_client(&keypair);
        let (_build_event_receiver, mut artifact_service) =
            create_artifact_service(&tmp_dir, &keypair, p2p_client.clone()).await;

        tokio::spawn(async move {
            loop {
                match command_receiver.recv().await {
                    Some(Command::ListPeers { sender, .. }) => {
                        let _ = sender.send(HashSet::new());
                    },
                    Some(Command::ListProviders { sender, .. }) => {
                        let mut set = HashSet::new();
                        set.insert(p2p_client.local_peer_id);
                        let _ = sender.send(set);
                    },
                    Some(Command::RequestIdleMetric { sender, .. }) => {
                        let _ = sender.send(Ok(PeerMetrics {
                            idle_metric: (0.1_f64).to_le_bytes()
                        }));
                    },
                    Some(Command::RequestArtifact { sender, .. }) => {
                        let _ = sender.send(Ok(b"SAMPLE_DATA".to_vec()));
                    },
                    _ => panic!("Command must match Command::ListPeers, Command::ListProviders, Command::RequestIdleMetric, Command::RequestArtifact"),
                }
            }
        });

        let mut hasher = Sha256::new();
        hasher.update(b"SAMPLE_DATA");
        let random_hash = hex::encode(hasher.finalize());

        let package_type = PackageType::Docker;
        let package_specific_id = "package_specific_id";
        let package_specific_artifact_id = "package_specific_artifact_id";
        let transparency_log = artifact_service
            .transparency_log_service
            .add_artifact(AddArtifactRequest {
                package_type,
                package_specific_id: package_specific_id.to_owned(),
                num_artifacts: 8,
                package_specific_artifact_id: package_specific_artifact_id.to_owned(),
                artifact_hash: random_hash.clone(),
            })
            .await
            .unwrap();
        artifact_service
            .transparency_log_service
            .write_transparency_log(&transparency_log)
            .unwrap();

        let future = {
            artifact_service
                .get_artifact(package_type, package_specific_artifact_id)
                .await
        };
        let result = task::spawn_blocking(|| future).await.unwrap();
        assert!(result.is_ok());

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_get_from_peers_with_no_providers() {
        let tmp_dir = test_util::tests::setup();

        let keypair = Keypair::generate();

        let (mut command_receiver, p2p_client) = create_p2p_client(&keypair);
        let (_build_event_receiver, mut artifact_service) =
            create_artifact_service(&tmp_dir, &keypair, p2p_client.clone()).await;

        tokio::spawn(async move {
            tokio::select! {
                command = command_receiver.recv() => {
                    match command {
                        Some(Command::ListProviders { sender, .. }) => {
                            let _ = sender.send(Default::default());
                        },
                        _ => panic!("Command must match Command::ListProviders"),
                    }
                }
            }
        });

        let mut hasher = Sha256::new();
        hasher.update(b"SAMPLE_DATA");
        let hash_bytes = hasher.finalize();
        let artifact_id = hex::encode(hash_bytes);

        let future = { artifact_service.get_artifact_from_peers(&artifact_id).await };
        let result = task::spawn_blocking(|| future).await.unwrap();
        assert!(result.is_err());

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_verify_artifact_succeeds_when_hashes_same() {
        let tmp_dir = test_util::tests::setup();

        let keypair = Keypair::generate();

        let (mut command_receiver, p2p_client) = create_p2p_client(&keypair);
        let (_build_event_receiver, mut artifact_service) =
            create_artifact_service(&tmp_dir, &keypair, p2p_client.clone()).await;

        tokio::spawn(async move {
            loop {
                match command_receiver.recv().await {
                    Some(Command::ListPeers { sender, .. }) => {
                        let _ = sender.send(HashSet::new());
                    }
                    _ => panic!("Command must match Command::ListPeers"),
                }
            }
        });

        let mut hasher1 = Sha256::new();
        hasher1.update(b"SAMPLE_DATA");
        let random_hash = hex::encode(hasher1.finalize());

        let package_type = PackageType::Docker;
        let package_specific_id = "package_specific_id";
        let package_specific_artifact_id = "package_specific_artifact_id";
        let created_transparency_log = artifact_service
            .transparency_log_service
            .add_artifact(AddArtifactRequest {
                package_type,
                package_specific_id: package_specific_id.to_owned(),
                num_artifacts: 8,
                package_specific_artifact_id: package_specific_artifact_id.to_owned(),
                artifact_hash: random_hash,
            })
            .await
            .unwrap();
        artifact_service
            .transparency_log_service
            .write_transparency_log(&created_transparency_log)
            .unwrap();

        let transparency_log = artifact_service
            .transparency_log_service
            .get_artifact(&package_type, package_specific_artifact_id)
            .unwrap();

        let result = artifact_service
            .verify_artifact(&transparency_log, b"SAMPLE_DATA")
            .await;
        assert!(result.is_ok());

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_verify_artifact_fails_when_hashes_differ() {
        let tmp_dir = test_util::tests::setup();

        let keypair = Keypair::generate();

        let (mut command_receiver, p2p_client) = create_p2p_client(&keypair);
        let (_build_event_receiver, mut artifact_service) =
            create_artifact_service(&tmp_dir, &keypair, p2p_client.clone()).await;

        tokio::spawn(async move {
            loop {
                match command_receiver.recv().await {
                    Some(Command::ListPeers { sender, .. }) => {
                        let _ = sender.send(HashSet::new());
                    }
                    _ => panic!("Command must match Command::ListPeers"),
                }
            }
        });

        let mut hasher1 = Sha256::new();
        hasher1.update(b"SAMPLE_DATA");
        let random_hash = hex::encode(hasher1.finalize());

        let mut hasher2 = Sha256::new();
        hasher2.update(b"OTHER_SAMPLE_DATA");
        let random_other_hash = hex::encode(hasher2.finalize());

        let package_type = PackageType::Docker;
        let package_specific_id = "package_specific_id";
        let package_specific_artifact_id = "package_specific_artifact_id";
        let created_transparency_log = artifact_service
            .transparency_log_service
            .add_artifact(AddArtifactRequest {
                package_type,
                package_specific_id: package_specific_id.to_owned(),
                num_artifacts: 8,
                package_specific_artifact_id: package_specific_artifact_id.to_owned(),
                artifact_hash: random_hash.clone(),
            })
            .await
            .unwrap();
        artifact_service
            .transparency_log_service
            .write_transparency_log(&created_transparency_log)
            .unwrap();

        let transparency_log = artifact_service
            .transparency_log_service
            .get_artifact(&package_type, package_specific_artifact_id)
            .unwrap();

        let verify_error = artifact_service
            .verify_artifact(&transparency_log, b"OTHER_SAMPLE_DATA")
            .await
            .expect_err("Verify artifact should have failed.");
        match verify_error {
            TransparencyLogError::InvalidHash {
                id,
                invalid_hash,
                actual_hash,
            } => {
                assert_eq!(id, package_specific_artifact_id.to_string());
                assert_eq!(invalid_hash, random_other_hash);
                assert_eq!(actual_hash, random_hash);
            }
            e => {
                panic!("Invalid Error encountered: {:?}", e);
            }
        }

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_get_artifact_logs() {
        let tmp_dir = test_util::tests::setup();

        let keypair = Keypair::generate();

        let (mut command_receiver, p2p_client) = create_p2p_client(&keypair);
        let (_build_event_receiver, mut artifact_service) =
            create_artifact_service(&tmp_dir, &keypair, p2p_client.clone()).await;

        tokio::spawn(async move {
            loop {
                match command_receiver.recv().await {
                    Some(Command::ListPeers { sender, .. }) => {
                        let _ = sender.send(HashSet::new());
                    }
                    _ => panic!("Command must match Command::ListPeers"),
                }
            }
        });

        let hasher1 = Sha256::new();
        let random_hash = hex::encode(hasher1.finalize());

        let package_type = PackageType::Maven2;
        let package_specific_id = "package_specific_id";
        let package_specific_artifact_id = "package_specific_artifact_id";
        let transparency_log = artifact_service
            .transparency_log_service
            .add_artifact(AddArtifactRequest {
                package_type,
                package_specific_id: package_specific_id.to_owned(),
                num_artifacts: 8,
                package_specific_artifact_id: package_specific_artifact_id.to_owned(),
                artifact_hash: random_hash,
            })
            .await
            .unwrap();
        artifact_service
            .transparency_log_service
            .write_transparency_log(&transparency_log)
            .unwrap();

        let result = artifact_service
            .transparency_log_service
            .search_transparency_logs(&package_type, package_specific_id);

        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 1);

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_request_build_without_authorized_nodes() {
        let tmp_dir = test_util::tests::setup();

        let keypair = Keypair::generate();

        let (_command_receiver, p2p_client) = create_p2p_client(&keypair);
        let (_build_event_receiver, artifact_service) =
            create_artifact_service(&tmp_dir, &keypair, p2p_client.clone()).await;

        let package_type = PackageType::Docker;
        let package_specific_id = "package_specific_id";

        // request build
        let error = artifact_service
            .request_build(package_type, package_specific_id.to_string())
            .await
            .unwrap_err();

        assert_eq!(
            error,
            BuildError::InitializationFailed("No authorized nodes found".to_owned())
        );

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_request_build_starts_on_local_authorized_node() {
        let tmp_dir = test_util::tests::setup();

        let keypair = Keypair::generate();

        let (mut command_receiver, p2p_client) = create_p2p_client(&keypair);
        let (mut build_event_receiver, artifact_service) =
            create_artifact_service(&tmp_dir, &keypair, p2p_client.clone()).await;

        tokio::spawn(async move {
            loop {
                match command_receiver.recv().await {
                    Some(Command::ListPeers { sender, .. }) => {
                        let _ = sender.send(HashSet::new());
                    }
                    _ => panic!("Command must match Command::ListPeers"),
                }
            }
        });

        tokio::spawn(async move {
            loop {
                match build_event_receiver.recv().await {
                    Some(BuildEvent::Start { sender, .. }) => {
                        let _ = sender.send(Ok(String::from("build_start_ok")));
                    }
                    _ => panic!("BuildEvent must match BuildEvent::Start"),
                }
            }
        });

        artifact_service
            .transparency_log_service
            .add_authorized_node(p2p_client.local_peer_id)
            .await
            .unwrap();

        let package_type = PackageType::Docker;
        let package_specific_id = "package_specific_id";

        // request build
        let result = artifact_service
            .request_build(package_type, package_specific_id.to_string())
            .await
            .unwrap();

        assert_eq!(result, String::from("build_start_ok"));

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_request_build_starts_on_other_authorized_node() {
        let tmp_dir = test_util::tests::setup();

        let keypair = Keypair::generate();

        let (mut command_receiver, p2p_client) = create_p2p_client(&keypair);
        let (_build_event_receiver, artifact_service) =
            create_artifact_service(&tmp_dir, &keypair, p2p_client.clone()).await;

        tokio::spawn(async move {
            loop {
                match command_receiver.recv().await {
                    Some(Command::ListPeers { sender, .. }) => {
                        let _ = sender.send(HashSet::new());
                    }
                    Some(Command::RequestBuild { sender, .. }) => {
                        let _ = sender.send(Ok(String::from("request_build_ok")));
                    }
                    other => panic!(
                        "Command must match Command::ListPeers or Command::RequestBuild, was: {:?}",
                        other
                    ),
                }
            }
        });

        let other_peer_id = PublicKey::Ed25519(Keypair::generate().public()).to_peer_id();

        artifact_service
            .transparency_log_service
            .add_authorized_node(other_peer_id)
            .await
            .unwrap();

        let package_type = PackageType::Docker;
        let package_specific_id = "package_specific_id";

        // request build
        let result = artifact_service
            .request_build(package_type, package_specific_id.to_string())
            .await
            .unwrap();

        assert_eq!(result, String::from("request_build_ok"));

        test_util::tests::teardown(tmp_dir);
    }

    fn get_file_reader() -> Result<File, anyhow::Error> {
        // test artifact file in resources/test dir
        let mut curr_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        curr_dir.push("tests/resources/artifact_test.json");

        let path = String::from(curr_dir.to_string_lossy());
        let reader = File::open(path.as_str()).unwrap();
        Ok(reader)
    }
}
