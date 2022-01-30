extern crate std;
extern crate libp2p;

use std::cell::RefCell;
use std::sync::Mutex;
use libp2p::kad::Kademlia;
use libp2p::kad::store::MemoryStore;
use crate::node_manager::handlers::LOCAL_PEER_ID;

/// A thread safe proxy to insure that only one thread at a time is making a call to kademlia. It uses internal mutability so that the caller of this struct's methods can use an immutable reference.
pub struct KademliaThreadSafeProxy {
    mutex: Mutex<RefCell<Kademlia<MemoryStore>>>
}

impl KademliaThreadSafeProxy {
    pub fn new() -> KademliaThreadSafeProxy {
        let kad = Kademlia::new(*LOCAL_PEER_ID, MemoryStore::new(*LOCAL_PEER_ID));
        KademliaThreadSafeProxy{mutex: Mutex::new(RefCell::new(kad))}
    }

    /// return true if the mutex is in a poisoned state due to a previous panic.
    pub fn is_poisoned(&self) -> bool {
        self.mutex.is_poisoned()
    }
}

#[cfg(test)]
mod tests {
    pub use super::*;

    #[test]
    pub fn new_proxy_test() {
        let proxy = KademliaThreadSafeProxy::new();
        assert!(!proxy.is_poisoned())
    }
}
