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

use super::storage::ArtifactStorage;
use crate::network::client::Client;
use crate::transparency_log::log::{TransparencyLog, TransparencyLogError, TransparencyLogService};
use anyhow::{bail, Context};
use libp2p::PeerId;
use log::info;
use multihash::Hasher;
use serde::{Deserialize, Serialize};
use std::io::{BufReader, Read};
use std::path::Path;
use std::str;

#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    PartialEq,
    Serialize,
    strum_macros::Display,
    strum_macros::EnumString,
)]
pub enum PackageType {
    Docker,
    Maven2,
}

/// The artifact service is the component that handles everything related to
/// pyrsia artifacts. It allows artifacts to be retrieved and added to the
/// pyrsia network by requesting a build from source.
pub struct ArtifactService {
    pub artifact_storage: ArtifactStorage,
    pub transparency_log_service: TransparencyLogService,
    pub p2p_client: Client,
}

impl ArtifactService {
    pub fn new<P: AsRef<Path>>(artifact_path: P, p2p_client: Client) -> anyhow::Result<Self> {
        let artifact_storage = ArtifactStorage::new(&artifact_path)?;
        let transparency_log_service = TransparencyLogService::new(&artifact_path)?;
        Ok(ArtifactService {
            artifact_storage,
            transparency_log_service,
            p2p_client,
        })
    }

    /// Request a build from source for the specified package.
    pub fn request_build(&self, _package_type: PackageType, _package_specific_id: &str) {}

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

        let artifact = match self.get_artifact_locally(&transparency_log.artifact_id) {
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
    pub fn get_artifact_locally(&mut self, artifact_id: &str) -> Result<Vec<u8>, anyhow::Error> {
        let artifact = self.artifact_storage.pull_artifact(artifact_id)?;
        let mut buf_reader = BufReader::new(artifact);
        let mut blob_content = Vec::new();
        buf_reader.read_to_end(&mut blob_content)?;
        Ok(blob_content)
    }

    async fn get_artifact_from_peers(
        &mut self,
        artifact_id: &str,
    ) -> Result<Vec<u8>, anyhow::Error> {
        let providers = self.p2p_client.list_providers(artifact_id).await?;

        match self.p2p_client.get_idle_peer(providers).await? {
            Some(peer) => self.get_artifact_from_peer(&peer, artifact_id).await,
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
        self.get_artifact_locally(artifact_id)
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

    /// Given artifact_id & reader, push artifact to artifact_manager
    /// and returns the boolean as true or false if it was able to create or not
    fn put_artifact(&self, artifact_id: &str, reader: &mut impl Read) -> Result<(), anyhow::Error> {
        info!("put_artifact with id: {}", artifact_id);
        self.artifact_storage
            .push_artifact(reader, artifact_id)
            .context("Error from put_artifact")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::client::command::Command;
    use crate::network::idle_metric_protocol::PeerMetrics;
    use crate::transparency_log::log::{AddArtifactRequest, TransparencyLogError};
    use crate::util::test_util;
    use anyhow::Context;
    use libp2p::identity::Keypair;
    use sha2::{Digest, Sha256};
    use std::collections::HashSet;
    use std::env;
    use std::fs::File;
    use std::path::PathBuf;
    use tokio::sync::{mpsc, oneshot};
    use tokio::task;

    const VALID_ARTIFACT_HASH: [u8; 32] = [
        0x86, 0x5c, 0x8d, 0x98, 0x8b, 0xe4, 0x66, 0x9f, 0x3e, 0x48, 0xf7, 0x3b, 0x98, 0xf9, 0xbc,
        0x25, 0x7, 0xbe, 0x2, 0x46, 0xea, 0x35, 0xe0, 0x9, 0x8c, 0xf6, 0x5, 0x4d, 0x36, 0x44, 0xc1,
        0x4f,
    ];

    #[tokio::test]
    async fn test_put_and_get_artifact() {
        let tmp_dir = test_util::tests::setup();

        let (add_artifact_sender, add_artifact_receiver) = oneshot::channel();
        let (sender, _receiver) = mpsc::channel(1);
        let p2p_client = Client {
            sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
        };

        let mut artifact_service = ArtifactService::new(&tmp_dir, p2p_client).unwrap();

        let package_type = PackageType::Docker;
        let package_specific_id = "package_specific_id";
        let package_specific_artifact_id = "package_specific_artifact_id";
        artifact_service
            .transparency_log_service
            .add_artifact(
                AddArtifactRequest {
                    package_type,
                    package_specific_id: package_specific_id.to_owned(),
                    package_specific_artifact_id: package_specific_artifact_id.to_owned(),
                    artifact_hash: hex::encode(VALID_ARTIFACT_HASH),
                    source_hash: hex::encode(VALID_ARTIFACT_HASH),
                },
                add_artifact_sender,
            )
            .await
            .unwrap();

        let transparency_log = add_artifact_receiver.await.unwrap().unwrap();

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

        let (add_artifact_sender, add_artifact_receiver) = oneshot::channel();
        let (sender, mut receiver) = mpsc::channel(1);
        let local_peer_id = Keypair::generate_ed25519().public().to_peer_id();
        let p2p_client = Client {
            sender,
            local_peer_id,
        };

        tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    Some(Command::ListProviders { sender, .. }) => {
                        let mut set = HashSet::new();
                        set.insert(local_peer_id);
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
                    _ => panic!("Command must match Command::ListProviders, Command::RequestIdleMetric, Command::RequestArtifact"),
                }
            }
        });

        let mut artifact_service = ArtifactService::new(&tmp_dir, p2p_client).unwrap();

        let mut hasher = Sha256::new();
        hasher.update(b"SAMPLE_DATA");
        let random_hash = hex::encode(hasher.finalize());

        let package_type = PackageType::Docker;
        let package_specific_id = "package_specific_id";
        let package_specific_artifact_id = "package_specific_artifact_id";
        artifact_service
            .transparency_log_service
            .add_artifact(
                AddArtifactRequest {
                    package_type,
                    package_specific_id: package_specific_id.to_owned(),
                    package_specific_artifact_id: package_specific_artifact_id.to_owned(),
                    artifact_hash: random_hash.clone(),
                    source_hash: random_hash.clone(),
                },
                add_artifact_sender,
            )
            .await
            .unwrap();

        add_artifact_receiver.await.unwrap().unwrap();

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

        let (sender, mut receiver) = mpsc::channel(1);
        let peer_id = Keypair::generate_ed25519().public().to_peer_id();
        let p2p_client = Client {
            sender,
            local_peer_id: peer_id,
        };

        let mut artifact_service = ArtifactService::new(&tmp_dir, p2p_client).unwrap();

        tokio::spawn(async move {
            tokio::select! {
                command = receiver.recv() => {
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

        let (add_artifact_sender, _receiver) = oneshot::channel();
        let (sender, _receiver) = mpsc::channel(1);
        let local_peer_id = Keypair::generate_ed25519().public().to_peer_id();
        let p2p_client = Client {
            sender,
            local_peer_id,
        };

        let mut artifact_service = ArtifactService::new(&tmp_dir, p2p_client).unwrap();

        let mut hasher1 = Sha256::new();
        hasher1.update(b"SAMPLE_DATA");
        let random_hash = hex::encode(hasher1.finalize());

        let package_type = PackageType::Docker;
        let package_specific_id = "package_specific_id";
        let package_specific_artifact_id = "package_specific_artifact_id";
        artifact_service
            .transparency_log_service
            .add_artifact(
                AddArtifactRequest {
                    package_type,
                    package_specific_id: package_specific_id.to_owned(),
                    package_specific_artifact_id: package_specific_artifact_id.to_owned(),
                    artifact_hash: random_hash.clone(),
                    source_hash: random_hash,
                },
                add_artifact_sender,
            )
            .await
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

        let (add_artifact_sender, _receiver) = oneshot::channel();
        let (sender, _receiver) = mpsc::channel(1);
        let local_peer_id = Keypair::generate_ed25519().public().to_peer_id();
        let p2p_client = Client {
            sender,
            local_peer_id,
        };

        let mut artifact_service = ArtifactService::new(&tmp_dir, p2p_client).unwrap();

        let mut hasher1 = Sha256::new();
        hasher1.update(b"SAMPLE_DATA");
        let random_hash = hex::encode(hasher1.finalize());

        let mut hasher2 = Sha256::new();
        hasher2.update(b"OTHER_SAMPLE_DATA");
        let random_other_hash = hex::encode(hasher2.finalize());

        let package_type = PackageType::Docker;
        let package_specific_id = "package_specific_id";
        let package_specific_artifact_id = "package_specific_artifact_id";
        artifact_service
            .transparency_log_service
            .add_artifact(
                AddArtifactRequest {
                    package_type,
                    package_specific_id: package_specific_id.to_owned(),
                    package_specific_artifact_id: package_specific_artifact_id.to_owned(),
                    artifact_hash: random_hash.clone(),
                    source_hash: random_hash.clone(),
                },
                add_artifact_sender,
            )
            .await
            .unwrap();

        let transparency_log = artifact_service
            .transparency_log_service
            .get_artifact(&package_type, package_specific_artifact_id)
            .unwrap();

        let result = artifact_service
            .verify_artifact(&transparency_log, b"OTHER_SAMPLE_DATA")
            .await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            TransparencyLogError::InvalidHash {
                id: package_specific_artifact_id.to_string(),
                invalid_hash: random_other_hash,
                actual_hash: random_hash
            }
        );

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
