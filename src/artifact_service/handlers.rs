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

use crate::artifact_service::service::{Hash, HashAlgorithm};
use crate::artifact_service::storage::ArtifactStorage;
use crate::cli_commands::config::get_config;
use crate::network::client::{ArtifactType, Client};
use crate::transparency_log::log::{TransparencyLog, TransparencyLogError};
use anyhow::{bail, Context};
use byte_unit::Byte;
use futures::lock::Mutex;
use libp2p::PeerId;
use log::{debug, info};
use multihash::Hasher;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::str;
use std::sync::Arc;
use sysinfo::{NetworkExt, ProcessExt, System, SystemExt};

//TODO: read from CLI config file
pub const ALLOCATED_SPACE_FOR_ARTIFACTS: &str = "10.84 GB";

//peer metric constants
const CPU_STRESS_WEIGHT: f64 = 2_f64;
const NETWORK_STRESS_WEIGHT: f64 = 0.001_f64;
const DISK_STRESS_WEIGHT: f64 = 0.001_f64;

//get_artifact: given artifact_hash(artifactName) pulls artifact for  artifact_manager and
//              returns read object to read the bytes of artifact
pub async fn get_artifact(
    transparency_log: Arc<Mutex<TransparencyLog>>,
    p2p_client: Client,
    artifact_storage: &ArtifactStorage,
    namespace_specific_id: &str,
) -> anyhow::Result<Vec<u8>> {
    let artifact_id = transparency_log
        .lock()
        .await
        .get_artifact(namespace_specific_id)?;

    let blob_content = match get_artifact_locally(artifact_storage, &artifact_id) {
        Ok(blob_content) => Ok(blob_content),
        Err(_) => get_artifact_from_peers(p2p_client, artifact_storage, &artifact_id).await,
    }?;

    verify_artifact(transparency_log, namespace_specific_id, &blob_content).await?;

    Ok(blob_content)
}

pub fn get_artifact_locally(
    artifact_storage: &ArtifactStorage,
    artifact_id: &str,
) -> Result<Vec<u8>, anyhow::Error> {
    let decoded_hash = hex::decode(artifact_id)?;
    let hash: Hash = Hash::new(HashAlgorithm::SHA256, &decoded_hash)?;
    let result = artifact_storage.pull_artifact(&hash)?;
    let mut buf_reader: BufReader<File> = BufReader::new(result);
    let mut blob_content = Vec::new();
    buf_reader.read_to_end(&mut blob_content)?;
    Ok(blob_content)
}

async fn get_artifact_from_peers(
    mut p2p_client: Client,
    artifact_storage: &ArtifactStorage,
    artifact_id: &str,
) -> Result<Vec<u8>, anyhow::Error> {
    let providers = p2p_client
        .list_providers(ArtifactType::Artifact, artifact_id.into())
        .await?;

    match p2p_client.get_idle_peer(providers).await? {
        Some(peer) => {
            get_artifact_from_peer(p2p_client, artifact_storage, &peer, artifact_id).await
        }
        None => bail!(
            "Artifact with id {} is not available on the p2p network.",
            artifact_id
        ),
    }
}

async fn get_artifact_from_peer(
    mut p2p_client: Client,
    artifact_storage: &ArtifactStorage,
    peer_id: &PeerId,
    artifact_id: &str,
) -> Result<Vec<u8>, anyhow::Error> {
    let artifact = p2p_client
        .request_artifact(peer_id, ArtifactType::Artifact, artifact_id.into())
        .await?;

    let decoded_hash = hex::decode(artifact_id)?;
    let hash: Hash = Hash::new(HashAlgorithm::SHA256, &decoded_hash)?;
    let cursor = Box::new(std::io::Cursor::new(artifact));
    put_artifact(artifact_storage, &hash, cursor)?;
    get_artifact_locally(artifact_storage, artifact_id)
}

async fn verify_artifact(
    transparency_log: Arc<Mutex<TransparencyLog>>,
    namespace_specific_id: &str,
    blob_content: &[u8],
) -> Result<(), TransparencyLogError> {
    let mut sha256 = multihash::Sha2_256::default();
    sha256.update(blob_content);
    let calculated_hash = hex::encode(sha256.finalize());
    transparency_log
        .lock()
        .await
        .verify_artifact(namespace_specific_id, &calculated_hash)
}

//put_artifact: given artifact_hash(artifactName) & artifact_path push artifact to artifact_manager
//              and returns the boolean as true or false if it was able to create or not
pub fn put_artifact(
    artifact_storage: &ArtifactStorage,
    artifact_hash: &Hash,
    art_reader: Box<dyn Read>,
) -> Result<(), anyhow::Error> {
    info!("put_artifact hash: {}", artifact_hash);
    let mut buf_reader = BufReader::new(art_reader);
    artifact_storage
        .push_artifact(&mut buf_reader, artifact_hash)
        .context("Error from put_artifact")
}

pub fn get_arts_summary(
    artifact_storage: &ArtifactStorage,
) -> Result<HashMap<String, usize>, anyhow::Error> {
    artifact_storage
        .artifacts_count_bydir()
        .context("Error while getting artifacts count")
}

pub fn get_space_available(artifact_storage: &ArtifactStorage) -> Result<u64, anyhow::Error> {
    let disk_used_bytes = artifact_storage.space_used()?;

    let mut available_space: u64 = 0;
    let cli_config = get_config().context("Error getting cli config file")?;

    let total_allocated_size: u64 = Byte::from_str(cli_config.disk_allocated)
        .unwrap()
        .get_bytes();

    if total_allocated_size > disk_used_bytes {
        available_space = total_allocated_size - disk_used_bytes;
    }
    Ok(available_space)
}

pub fn disk_usage(artifact_storage: &ArtifactStorage) -> Result<f64, anyhow::Error> {
    let disk_used_bytes = artifact_storage.space_used()?;
    let cli_config = get_config().context("Error getting cli config file")?;
    let total_allocated_size: u64 = Byte::from_str(cli_config.disk_allocated)
        .unwrap()
        .get_bytes();
    let mut disk_usage: f64 = 0.0;
    debug!("disk_used: {}", disk_used_bytes);
    debug!("total_allocated_size: {}", total_allocated_size);

    if total_allocated_size > disk_used_bytes {
        disk_usage = (disk_used_bytes as f64 / total_allocated_size as f64) * 100_f64;
    }
    Ok(disk_usage)
}

/***************************************************
 * Peer Quality Metrics
 ***************************************************/
// Get the local stress metric to advertise to peers
pub fn get_quality_metric() -> f64 {
    let mut qm = get_cpu_stress() * CPU_STRESS_WEIGHT;
    qm += get_network_stress() * NETWORK_STRESS_WEIGHT;
    qm + get_disk_stress() * DISK_STRESS_WEIGHT
}

// This function gets the current CPU load on the system.
fn get_cpu_stress() -> f64 {
    let sys = System::new_all();
    let load_avg = sys.load_average();
    load_avg.one //using the average over the last 1 minute
}

//This function gets the current network load on the system
fn get_network_stress() -> f64 {
    let mut sys = System::new_all();
    sys.refresh_networks_list();
    let networks = sys.networks();

    let mut packets_in = 0;
    let mut packets_out = 0;
    for (_interface_name, network) in networks {
        packets_in += network.received();
        packets_out += network.transmitted();
    }
    (packets_in as f64) + (packets_out as f64)
    //TODO: add network card capabilities to the metric. cards with > network capacity should get a lower stress number.
}

fn get_disk_stress() -> f64 {
    let sys = System::new_all();
    // Sum up the disk usage measured as total read and writes per process:
    let mut total_usage = 0_u64;
    for process in sys.processes().values() {
        let usage = process.disk_usage();
        total_usage = total_usage + usage.total_written_bytes + usage.total_read_bytes;
    }
    total_usage as f64
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::client::command::Command;
    use crate::network::idle_metric_protocol::PeerMetrics;
    use crate::transparency_log::log::TransparencyLogError;
    use crate::util::test_util;
    use anyhow::Context;
    use assay::assay;
    use futures::channel::mpsc;
    use futures::executor;
    use futures::prelude::*;
    use libp2p::identity::Keypair;
    use sha2::{Digest, Sha256};
    use std::collections::HashSet;
    use std::env;
    use std::fs::File;
    use std::path::PathBuf;

    const VALID_ARTIFACT_HASH: [u8; 32] = [
        0x86, 0x5c, 0x8d, 0x98, 0x8b, 0xe4, 0x66, 0x9f, 0x3e, 0x48, 0xf7, 0x3b, 0x98, 0xf9, 0xbc,
        0x25, 0x7, 0xbe, 0x2, 0x46, 0xea, 0x35, 0xe0, 0x9, 0x8c, 0xf6, 0x5, 0x4d, 0x36, 0x44, 0xc1,
        0x4f,
    ];
    const CPU_THREADS: usize = 200;
    const NETWORK_THREADS: usize = 10;

    #[assay(
        env = [
          ("PYRSIA_ARTIFACT_PATH", "pyrsia-test-node"),
          ("DEV_MODE", "on")
        ],
        teardown = test_util::tear_down()
    )]
    #[tokio::test]
    async fn test_put_and_get_artifact() {
        let transparency_log = Arc::new(Mutex::new(TransparencyLog::new()));
        let artifact_storage = ArtifactStorage::new()?;

        let (sender, _) = mpsc::channel(1);
        let p2p_client = Client {
            sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
        };

        let artifact_id = "an_artifact_id";
        transparency_log
            .lock()
            .await
            .add_artifact(artifact_id, &hex::encode(VALID_ARTIFACT_HASH))?;

        let hash = Hash::new(HashAlgorithm::SHA256, &VALID_ARTIFACT_HASH)?;
        //put the artifact
        put_artifact(&artifact_storage, &hash, Box::new(get_file_reader()?))
            .context("Error from put_artifact")?;

        // pull artifact
        let future = async {
            get_artifact(
                transparency_log,
                p2p_client,
                &artifact_storage,
                &artifact_id,
            )
            .await
            .context("Error from get_artifact")
        };
        let file = executor::block_on(future)?;

        //validate pulled artifact with the actual data
        let mut s = String::new();
        get_file_reader()?.read_to_string(&mut s)?;

        let s1 = match str::from_utf8(file.as_slice()) {
            Ok(v) => v,
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        };
        assert_eq!(s, s1);
    }

    #[assay(
        env = [
            ("PYRSIA_ARTIFACT_PATH", "PyrsiaTest"),
            ("DEV_MODE", "on")
        ],
        teardown = test_util::tear_down()
    )]
    #[tokio::test]
    async fn test_get_from_peers() {
        let artifact_storage = ArtifactStorage::new()?;

        let peer_id = Keypair::generate_ed25519().public().to_peer_id();

        let (sender, mut receiver) = mpsc::channel(1);

        tokio::spawn(async move {
            loop {
                match receiver.next().await {
                    Some(Command::ListProviders { artifact_type: _artifact_type, artifact_hash: _artifact_hash, sender }) => {
                        let mut set = HashSet::new();
                        set.insert(peer_id);
                        let _ = sender.send(set);
                    },
                    Some(Command::RequestIdleMetric { peer: _peer, sender }) => {
                        let _ = sender.send(Ok(PeerMetrics {
                            idle_metric: (0.1_f64).to_le_bytes()
                        }));
                    },
                    Some(Command::RequestArtifact { artifact_type: _artifact_type, artifact_hash: _artifact_hash, peer: _peer, sender }) => {
                        let _ = sender.send(Ok(b"SAMPLE_DATA".to_vec()));
                    },
                    _ => panic!("Command must match Command::ListProviders, Command::RequestIdleMetric, Command::RequestArtifact"),
                }
            }
        });

        let p2p_client = Client {
            sender,
            local_peer_id: peer_id,
        };

        let mut hasher = Sha256::new();
        hasher.update(b"SAMPLE_DATA");
        let hash_bytes = hasher.finalize();
        let artifact_id = hex::encode(hash_bytes);

        let result = executor::block_on(async {
            get_artifact_from_peers(p2p_client, &artifact_storage, &artifact_id).await
        });
        assert!(result.is_ok());
    }

    #[assay(
        env = [
            ("PYRSIA_ARTIFACT_PATH", "PyrsiaTest"),
            ("DEV_MODE", "on")
        ],
        teardown = test_util::tear_down()
    )]
    #[tokio::test]
    async fn test_get_from_peers_with_no_providers() {
        let artifact_storage = ArtifactStorage::new()?;

        let peer_id = Keypair::generate_ed25519().public().to_peer_id();

        let (sender, mut receiver) = mpsc::channel(1);

        tokio::spawn(async move {
            futures::select! {
                command = receiver.next() => match command {
                    Some(Command::ListProviders { artifact_type: _artifact_type, artifact_hash: _artifact_hash, sender }) => {
                        let _ = sender.send(Default::default());
                    },
                    _ => panic!("Command must match Command::ListProviders"),
                }
            }
        });

        let p2p_client = Client {
            sender,
            local_peer_id: peer_id,
        };

        let mut hasher = Sha256::new();
        hasher.update(b"SAMPLE_DATA");
        let hash_bytes = hasher.finalize();
        let artifact_id = hex::encode(hash_bytes);

        let result = executor::block_on(async {
            get_artifact_from_peers(p2p_client, &artifact_storage, &artifact_id).await
        });
        assert!(result.is_err());
    }

    #[assay(
        env = [
            ("PYRSIA_ARTIFACT_PATH", "PyrsiaTest"),
            ("DEV_MODE", "on")
        ],
        teardown = test_util::tear_down()
    )]
    #[tokio::test]
    async fn test_verify_artifact_succeeds_when_hashes_same() {
        let mut hasher1 = Sha256::new();
        hasher1.update(b"SAMPLE_DATA");
        let random_hash = hex::encode(hasher1.finalize());

        let transparency_log = Arc::new(Mutex::new(TransparencyLog::new()));

        let namespace_specific_id = "namespace_specific_id";
        transparency_log
            .lock()
            .await
            .add_artifact(namespace_specific_id, &random_hash)?;

        let result =
            verify_artifact(transparency_log, &namespace_specific_id, b"SAMPLE_DATA").await;
        assert!(result.is_ok());
    }

    #[assay(
        env = [
            ("PYRSIA_ARTIFACT_PATH", "PyrsiaTest"),
            ("DEV_MODE", "on")
        ],
        teardown = test_util::tear_down()
    )]
    #[tokio::test]
    async fn test_verify_artifact_fails_when_hashes_differ() {
        let mut hasher1 = Sha256::new();
        hasher1.update(b"SAMPLE_DATA");
        let random_hash = hex::encode(hasher1.finalize());

        let mut hasher2 = Sha256::new();
        hasher2.update(b"OTHER_SAMPLE_DATA");
        let random_other_hash = hex::encode(hasher2.finalize());

        let transparency_log = Arc::new(Mutex::new(TransparencyLog::new()));

        let namespace_specific_id = "namespace_specific_id";
        transparency_log
            .lock()
            .await
            .add_artifact(namespace_specific_id, &random_hash)?;

        let result = verify_artifact(
            transparency_log,
            &namespace_specific_id,
            b"OTHER_SAMPLE_DATA",
        )
        .await;
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            TransparencyLogError::InvalidHash {
                id: namespace_specific_id.to_string(),
                invalid_hash: random_other_hash,
                actual_hash: random_hash
            }
        );
    }

    #[assay(
        env = [
          ("PYRSIA_ARTIFACT_PATH", "PyrsiaTest"),
          ("DEV_MODE", "on")
        ],
        teardown = test_util::tear_down()
    )]
    fn test_disk_usage() {
        let artifact_storage = ArtifactStorage::new()?;

        let usage_pct_before = disk_usage(&artifact_storage).context("Error from disk_usage")?;

        create_artifact(&artifact_storage).context("Error creating artifact")?;

        let usage_pct_after = disk_usage(&artifact_storage).context("Error from disk_usage")?;
        assert!(usage_pct_before < usage_pct_after);
    }

    #[assay(
        env = [
          ("PYRSIA_ARTIFACT_PATH", "PyrsiaTest"),
          ("DEV_MODE", "on")
        ],
        teardown = test_util::tear_down()
    )]
    fn test_get_space_available() {
        let artifact_storage = ArtifactStorage::new()?;

        let space_available_before =
            get_space_available(&artifact_storage).context("Error from get_space_available")?;

        create_artifact(&artifact_storage).context("Error creating artifact")?;

        let space_available_after =
            get_space_available(&artifact_storage).context("Error from get_space_available")?;
        debug!(
            "Before: {}; After: {}",
            space_available_before, space_available_after
        );
        assert!(space_available_after < space_available_before);
    }

    fn get_file_reader() -> Result<File, anyhow::Error> {
        // test artifact file in resources/test dir
        let mut curr_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        curr_dir.push("tests/resources/artifact_test.json");

        let path = String::from(curr_dir.to_string_lossy());
        let reader = File::open(path.as_str()).unwrap();
        Ok(reader)
    }

    fn create_artifact(artifact_storage: &ArtifactStorage) -> Result<(), anyhow::Error> {
        let hash = Hash::new(HashAlgorithm::SHA256, &VALID_ARTIFACT_HASH)?;
        artifact_storage
            .push_artifact(&mut get_file_reader()?, &hash)
            .context("Error while pushing artifact")
    }

    #[test]
    fn cpu_load_test() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use std::thread;
        use std::time::Duration;

        let loading = Arc::new(AtomicBool::new(true));

        //first measure of CPU for benchmark
        let qm = get_cpu_stress() * CPU_STRESS_WEIGHT;
        assert_ne!(0_f64, qm); //zero should never be returned here

        //set CPU on fire to measure stress
        let mut threads = vec![];
        for _i in 0..CPU_THREADS {
            threads.push(thread::spawn({
                let mut cpu_fire = 0;
                let loading_test = loading.clone();
                move || {
                    while loading_test.load(Ordering::Relaxed) {
                        cpu_fire = cpu_fire + 1;
                    }
                }
            }));
        }

        thread::sleep(Duration::from_millis(200)); //let cpu spin up

        //second measure of CPU
        let qm2 = get_cpu_stress() * CPU_STRESS_WEIGHT;
        assert!(qm2 >= qm);
        loading.store(false, Ordering::Relaxed); //kill threads

        //wait for threads
        for thread in threads {
            thread.join().unwrap();
        }
        //we could add another measure of CPU did no think it was that important
    }

    #[test]
    fn network_load_test() {
        use std::net::UdpSocket;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use std::thread;

        let loading = Arc::new(AtomicBool::new(true));

        //fist measure of network for benchmark
        let qm = get_network_stress() * NETWORK_STRESS_WEIGHT;

        //shotgun the network with packets
        let mut threads = vec![];
        for i in 0..NETWORK_THREADS {
            threads.push(thread::spawn({
                let address: String = format_args!("127.0.0.1:3425{i}").to_string();
                let socket = UdpSocket::bind(address).expect("couldn't bind to address");
                let loading_test = loading.clone();
                move || {
                    while loading_test.load(Ordering::Relaxed) {
                        socket
                            .send_to(&[0; 10], "127.0.0.1:4242")
                            .expect("couldn't send data");
                    }
                }
            }));
        }

        let qm2 = get_network_stress() * NETWORK_STRESS_WEIGHT;
        assert!(qm2 > qm);
        loading.store(false, Ordering::Relaxed); //kill threads

        //wait for threads
        for thread in threads {
            thread.join().unwrap();
        }
        //we could add another measure of network did no think it was that important
    }

    #[test]
    fn disk_load_test() {
        use std::fs;
        use std::fs::OpenOptions;
        use std::io::Write;
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;
        use std::thread;
        use std::time::Duration;

        let loading = Arc::new(AtomicBool::new(true));
        let test_file = "pyrsia_test.txt";

        // fist measure of network for benchmark
        let qm = get_disk_stress() * DISK_STRESS_WEIGHT;

        // write some data
        let write_thread = thread::spawn({
            let file_data = "Some test data for the file!\n";
            let except_str = format!("Unable to open file {}", test_file).to_string();
            let mut f = OpenOptions::new()
                .append(true)
                .create(true)
                .open(test_file)
                .expect(&except_str);
            let loading_test = loading.clone();
            move || {
                while loading_test.load(Ordering::Relaxed) {
                    f.write_all(file_data.as_bytes())
                        .expect("Unable to write data");
                }
                drop(f);
            }
        });

        thread::sleep(Duration::from_millis(400)); //let writes happen

        // second measure of network
        let qm2 = get_disk_stress() * DISK_STRESS_WEIGHT;
        loading.store(false, Ordering::Relaxed); //kill thread
        write_thread.join().unwrap();
        fs::remove_file(test_file).unwrap_or_else(|why| {
            assert!(false, "{:?}", why.kind());
        });
        assert!(qm2 > qm);

        //we could add another measure of disks did no think it was that important
    }

    #[test]
    fn quality_metric_test() {
        let quality_metric = get_quality_metric();
        assert!(quality_metric != 0_f64);
    }
}
