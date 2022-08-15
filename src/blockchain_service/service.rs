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

use libp2p::identity;
use pyrsia_blockchain_network::blockchain::Blockchain;
use pyrsia_blockchain_network::structures::block::Block;
use std::fmt::{self, Debug, Formatter};

use crate::network::client::Client;

pub struct BlockchainService {
    blockchain: Blockchain,
    p2p_client: Client,
}

impl Debug for BlockchainService {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("BlockchainService")
            .field("blockchain", &self.blockchain)
            .field("p2p_client", &self.p2p_client)
            .finish()
    }
}

impl BlockchainService {
    pub fn new(keypair: &identity::ed25519::Keypair, p2p_client: Client) -> Self {
        Self {
            blockchain: Blockchain::new(keypair),
            p2p_client,
        }
    }

     /// Add payload to blockchain
     pub async fn add_payload(
        &mut self,
        payload: Vec<u8>,
        local_key: &identity::Keypair) {
            let _ = self.blockchain.add_block(payload, local_key).await;

            self.notify_block_update(self.blockchain.last_block().unwrap()).await;
    }

    pub async fn notify_block_update(&mut self, block:Block) {
        let peer_list =  self.p2p_client.list_peers().await.unwrap();
        let block_ordinal = block.header.ordinal.clone();
        for peer_id in peer_list.iter(){
            let _ = self.p2p_client.request_block_update(&peer_id, block_ordinal, block.clone() ).await;

            
        }
    }


}
