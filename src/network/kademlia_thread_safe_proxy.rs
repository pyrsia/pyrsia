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
extern crate libp2p;
extern crate std;

use crate::node_manager::handlers::LOCAL_PEER_ID;
use libp2p::kad::kbucket::EntryView;
use libp2p::kad::store::MemoryStore;
use libp2p::kad::{kbucket, record, Addresses, Kademlia, QueryId, Quorum, Record};
use libp2p::{Multiaddr, PeerId};
use record::store::Error;
use std::cell::RefCell;
use std::sync::{Mutex, MutexGuard};

/// A thread safe proxy to insure that only one thread at a time is making a call to kademlia. It uses internal mutability so that the caller of this struct's methods can use an immutable reference.
pub struct KademliaThreadSafeProxy {
    mutex: Mutex<RefCell<Kademlia<MemoryStore>>>,
}

impl KademliaThreadSafeProxy {
    pub fn default() -> KademliaThreadSafeProxy {
        let kad = Kademlia::new(*LOCAL_PEER_ID, MemoryStore::new(*LOCAL_PEER_ID));
        KademliaThreadSafeProxy {
            mutex: Mutex::new(RefCell::new(kad)),
        }
    }

    /// return true if the mutex is in a poisoned state due to a previous panic.
    pub fn is_poisoned(&self) -> bool {
        self.mutex.is_poisoned()
    }

    fn ref_cell(&self) -> MutexGuard<RefCell<Kademlia<MemoryStore>>> {
        self.mutex
            .lock()
            .expect("KademliaThreadSafeProxy called after a panic during a previous call!")
    }

    pub fn get_closest_peers<K>(&self, key: K) -> QueryId
    where
        K: Into<kbucket::Key<K>> + Into<Vec<u8>> + Clone,
    {
        (*self.ref_cell()).borrow_mut().get_closest_peers(key)
    }

    pub fn get_record(&self, key: record::Key, quorum: Quorum) -> QueryId {
        (*self.ref_cell()).borrow_mut().get_record(key, quorum)
    }

    pub fn put_record(&self, record: Record, quorum: Quorum) -> Result<QueryId, Error> {
        (*self.ref_cell()).borrow_mut().put_record(record, quorum)
    }

    pub fn add_address(&self, peer: &PeerId, address: Multiaddr) {
        (*self.ref_cell()).borrow_mut().add_address(peer, address);
    }

    pub fn remove_address(
        &self,
        peer: &PeerId,
        address: &Multiaddr,
    ) -> Option<EntryView<kbucket::Key<PeerId>, Addresses>> {
        (*self.ref_cell())
            .borrow_mut()
            .remove_address(peer, address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_manager::handlers::KADEMLIA_PROXY;
    use libp2p::{build_multiaddr, PeerId};

    #[test]
    pub fn add_and_remove_peer_addresses() {
        let peer = PeerId::random();
        let address = build_multiaddr!(Ip4([10, 11, 12, 13]), Tcp(9999u16));
        assert!(KADEMLIA_PROXY.remove_address(&peer, &address).is_none());
        KADEMLIA_PROXY.add_address(&peer, address.clone());
        if KADEMLIA_PROXY.remove_address(&peer, &address).is_none() {
            panic!("No address was removed when there was an address to be removed.")
        }
    }

    #[test]
    pub fn new_proxy_test() {
        let proxy = KademliaThreadSafeProxy::default();
        assert!(!proxy.is_poisoned())
    }

    #[test]
    pub fn get_record_test_just_to_check_that_it_returns_a_different_query_id_each_call() {
        let key = record::Key::from(vec![0xde, 0xad, 0xbe, 0xef]);
        let q1 = KADEMLIA_PROXY.get_record(key.clone(), Quorum::One);
        let q2 = KADEMLIA_PROXY.get_record(key.clone(), Quorum::One);
        assert_ne!(q1, q2, "Query IDs should not be equal");
    }

    #[test]
    pub fn get_closest_peers_test_just_to_check_that_it_returns_a_different_query_id_each_call() {
        let peer_id: PeerId = *LOCAL_PEER_ID;
        let q1 = KADEMLIA_PROXY.get_closest_peers(peer_id);
        let q2 = KADEMLIA_PROXY.get_closest_peers(peer_id);
        assert_ne!(q1, q2, "Query IDs should not be equal");
    }

    #[test]
    pub fn put_record_test() -> Result<(), Error> {
        let record = Record {
            key: record::Key::from(vec![0xdeu8, 0xadu8, 0xbeu8, 0xefu8]),
            value: vec![0xf0u8, 0x83u8, 0x32u8],
            publisher: Some(*LOCAL_PEER_ID),
            expires: None,
        };
        let q1 = KADEMLIA_PROXY.put_record(record.clone(), Quorum::One)?;
        let q2 = KADEMLIA_PROXY.put_record(record, Quorum::One)?;
        assert_ne!(q1, q2, "Query IDs should not be equal");
        Ok(())
    }
}
