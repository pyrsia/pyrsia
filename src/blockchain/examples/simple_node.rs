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
use futures::channel::{mpsc as futures_mpsc, oneshot};
use futures::StreamExt;
use libp2p::{identity, PeerId};
use log::{debug, info};
use std::{
    error::Error,
    fs,
    io::{Read, Write},
    os::unix::fs::OpenOptionsExt,
    sync::{Arc, Mutex},
};
use tokio::io;

// use pyrsia_blockchain_network::blockchain::Blockchain;
use pyrsia_blockchain_network::args::parser::BlockchainNodeArgs;
use pyrsia_blockchain_network::crypto::hash_algorithm::HashDigest;
use pyrsia_blockchain_network::identities::{
    authority_pen::AuthorityPen, authority_verifier::AuthorityVerifier, key_box::KeyBox,
};
use pyrsia_blockchain_network::network::{Network, Spawner};
use pyrsia_blockchain_network::providers::{DataProvider, DataStore, FinalizationProvider};
use pyrsia_blockchain_network::structures::block::Block;
use pyrsia_blockchain_network::{
    default_config, gen_chain_config, run_blockchain, run_session, NodeIndex,
};

const TXS_PER_BLOCK: usize = 50000;
const TX_SIZE: usize = 300;
const BLOCK_TIME_MS: u128 = 500;
const INITIAL_DELAY_MS: u128 = 5000;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    let args = BlockchainNodeArgs::parse();

    let key_path = get_keyfile_name(args.clone());

    // If the key file exists, load the key pair. Otherwise, create a random keypair and save to the keypair file
    let id_keys = create_ed25519_keypair(key_path);
    let ed25519_pair = identity::Keypair::Ed25519(id_keys.clone());
    let _peer_id = PeerId::from(ed25519_pair.public());

    info!("Getting network up!");
    let n_members = 3;
    let my_node_ix = NodeIndex(args.peer_index);

    let pen = AuthorityPen::new(my_node_ix, id_keys.clone());
    let verifier = AuthorityVerifier::new();

    let keybox = KeyBox::new(pen, verifier);

    let (authority_to_verifier, mut authority_from_network) = futures_mpsc::unbounded();
    let (close_verifier, mut exit) = oneshot::channel();
    tokio::spawn(async move {
        loop {
            futures::select! {
                maybe_auth = authority_from_network.next() => {
                    if let Some((_node_ix, _public_key)) = maybe_auth {
                        // record_authority(node_ix, public_key);
                    }
                }
               _ = &mut exit  => break,
            }
        }
    });

    let (
        network,
        mut manager,
        block_from_data_io_tx,
        block_from_network_rx,
        message_for_network,
        message_from_network,
    ) = Network::new(
        my_node_ix,
        id_keys.clone(),
        Default::default(), // peers_by_index,
        authority_to_verifier,
    )
    .await
    .expect("Libp2p network set-up should succeed.");
    // Make the "genesis" blocks
    let current_block: Arc<Mutex<Block>> = Arc::new(Mutex::new(Block::new(
        HashDigest::new(b""),
        0,
        vec![],
        &id_keys,
    )));

    let data_provider = DataProvider::new(current_block.clone()); // TODO(prince-chrismc): Blend this into blockchain API???
    let (finalization_provider, mut finalized_rx) = FinalizationProvider::new();
    let data_store = DataStore::new(current_block.clone(), message_for_network);

    let (close_network, exit) = oneshot::channel();
    tokio::spawn(async move { manager.run(exit).await });

    let data_size: usize = TXS_PER_BLOCK * TX_SIZE;
    let chain_config = gen_chain_config(
        my_node_ix,
        n_members,
        data_size,
        BLOCK_TIME_MS,
        INITIAL_DELAY_MS,
    );
    let (close_chain, exit) = oneshot::channel();
    tokio::spawn(async move {
        run_blockchain(
            chain_config,
            data_store,
            current_block,
            block_from_network_rx,
            block_from_data_io_tx,
            message_from_network,
            exit,
        )
        .await
    });

    let (close_member, exit) = oneshot::channel();
    tokio::spawn(async move {
        let config = default_config(n_members.into(), my_node_ix, 0);
        run_session(
            config,
            network,
            data_provider,
            finalization_provider,
            keybox,
            Spawner {},
            exit,
        )
        .await
    });

    let mut max_block_finalized = 0;
    while let Some(block_num) = finalized_rx.next().await {
        if max_block_finalized < block_num.header.ordinal {
            max_block_finalized = block_num.header.ordinal;
        }
        debug!(
            "🌟 Got new batch. Highest finalized = {:?}",
            max_block_finalized
        );
        if max_block_finalized >= 100_u128 {
            break;
        }
    }
    close_member.send(()).expect("should send");
    close_chain.send(()).expect("should send");
    close_network.send(()).expect("should send");
    close_verifier.send(()).expect("should send");
    Ok(())
}

pub fn write_block(path: &str, block: Block) {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(path)
        .expect("cannot open file");

    file.write_all(serde_json::to_string(&block).unwrap().as_bytes())
        .expect("write failed");
    file.write_all(b"\n").expect("write failed");
}

pub fn write_keypair(path: &String, data: &[u8; 64]) {
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .mode(0o600)
        .open(path)
        .expect("cannot open file");

    file.write_all(data).expect("write failed");
}

pub fn read_keypair(path: &String) -> Result<[u8; 64], Box<dyn Error>> {
    let mut file = std::fs::File::open(path)?;
    let mut buf = [0u8; 64];
    let n = file.read(&mut buf)?;
    if n == 64 {
        Ok(buf)
    } else {
        Err(Box::new(io::Error::from(io::ErrorKind::InvalidData)))
    }
}

pub fn get_keyfile_name(args: BlockchainNodeArgs) -> String {
    let mut path = dirs::home_dir().unwrap();
    path.push(args.key_filename);
    path.into_os_string().into_string().unwrap()
}

pub fn create_ed25519_keypair(filename: String) -> libp2p::identity::ed25519::Keypair {
    match read_keypair(&filename) {
        Ok(v) => {
            let data: &mut [u8] = &mut v.clone();
            debug!("Load Keypair from {:?}", filename);
            libp2p::identity::ed25519::Keypair::decode(data).unwrap()
        }
        Err(_) => {
            let id_keys = identity::ed25519::Keypair::generate();

            let data = id_keys.encode();
            debug!("Create Keypair");
            write_keypair(&filename, &data);
            id_keys
        }
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use pyrsia_blockchain_network::args::parser::DEFAULT_BLOCK_KEYPAIR_FILENAME;
    const TEST_KEYPAIR_FILENAME: &str = "./test_keypair";
    #[test]
    fn test_get_keyfile_name_succeeded() {
        let mut path = dirs::home_dir().unwrap();

        path.push(DEFAULT_BLOCK_KEYPAIR_FILENAME);
        let args = BlockchainNodeArgs {
            key_filename: DEFAULT_BLOCK_KEYPAIR_FILENAME.to_string(),
            peer_index: 0,
        };
        assert_eq!(
            path.into_os_string().into_string().unwrap(),
            get_keyfile_name(args)
        );
    }

    #[test]
    fn test_write_keypair_succeeded() {
        let file = String::from(TEST_KEYPAIR_FILENAME);
        let data = [0u8; 64];
        let result = std::panic::catch_unwind(|| write_keypair(&file, &data));
        assert!(result.is_ok());
    }

    #[test]
    fn test_read_keypair_succeeded() {
        let file = String::from(TEST_KEYPAIR_FILENAME);
        let data = [0u8; 64];
        write_keypair(&file, &data);
        assert!(read_keypair(&file).is_ok());
    }

    #[test]
    fn test_create_keypair_succeeded() {
        let args = BlockchainNodeArgs {
            key_filename: DEFAULT_BLOCK_KEYPAIR_FILENAME.to_string(),
            peer_index: 0,
        };
        let result = std::panic::catch_unwind(|| create_ed25519_keypair(get_keyfile_name(args)));
        assert!(result.is_ok());
    }
}
