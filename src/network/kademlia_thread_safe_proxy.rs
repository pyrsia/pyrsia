extern crate libp2p;
extern crate std;

use crate::node_manager::handlers::LOCAL_PEER_ID;
use libp2p::kad::record::Key;
use libp2p::kad::store::MemoryStore;
use libp2p::kad::{Kademlia, kbucket, QueryId, Quorum};
use std::cell::RefCell;
use std::sync::{Mutex, MutexGuard};

/// A thread safe proxy to insure that only one thread at a time is making a call to kademlia. It uses internal mutability so that the caller of this struct's methods can use an immutable reference.
pub struct KademliaThreadSafeProxy {
    mutex: Mutex<RefCell<Kademlia<MemoryStore>>>,
}

impl KademliaThreadSafeProxy {
    pub fn new() -> KademliaThreadSafeProxy {
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

    pub fn get_record(&self, key: &Key, quorum: Quorum) -> QueryId {
        (*self.ref_cell()).borrow_mut().get_record(key, quorum)
    }
}

#[cfg(test)]
mod tests {
    pub use super::*;
    use crate::node_manager::handlers::KADEMLIA_PROXY;

    #[test]
    pub fn new_proxy_test() {
        let proxy = KademliaThreadSafeProxy::new();
        assert!(!proxy.is_poisoned())
    }

    #[test]
    pub fn get_record_test_just_to_check_that_it_returns_a_different_query_id_each_call() {
        let key = Key::from(vec![0xde, 0xad, 0xbe, 0xef]);
        let q1 = KADEMLIA_PROXY.get_record(&key, Quorum::One);
        let q2 = KADEMLIA_PROXY.get_record(&key, Quorum::One);
        assert_ne!(q1, q2, "Query IDs should not be equal");
    }

    #[test]
    pub fn get_closest_peers_test_just_to_check_that_it_returns_a_different_query_id_each_call() {
        let key = Key::from(vec![0xde, 0xad, 0xbe, 0xef]);
        let q1 = KADEMLIA_PROXY.get_closest_peers(&key);
        let q2 = KADEMLIA_PROXY.get_closest_peers(&key);
        assert_ne!(q1, q2, "Query IDs should not be equal");
    }
}
