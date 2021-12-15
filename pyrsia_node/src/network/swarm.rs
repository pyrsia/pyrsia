use super::behavior::MyBehaviour;
use super::transport::TcpTokioTransport;
use libp2p::{
    floodsub::{Floodsub, Topic},
    kad::{record::store::MemoryStore, Kademlia},
    mdns::Mdns,
    swarm::SwarmBuilder,
    PeerId, Swarm,
};

pub type MyBehaviourSwarm = Swarm<MyBehaviour>;

pub async fn new(
    topic: Topic,
    transport: TcpTokioTransport,
    peer_id: PeerId,
    response_sender: tokio::sync::mpsc::Sender<String>,
) -> Result<MyBehaviourSwarm, ()> {
    //create kad
    let store = MemoryStore::new(peer_id);
    let kademlia = Kademlia::new(peer_id, store);

    let mdns = Mdns::new(Default::default()).await.unwrap();
    let mut behaviour = MyBehaviour::new(Floodsub::new(peer_id), kademlia, mdns, response_sender);
    behaviour.floodsub_mut().subscribe(topic.clone());

    let swarm = SwarmBuilder::new(transport, behaviour, peer_id)
        // We want the connection background tasks to be spawned
        // onto the tokio runtime.
        .executor(Box::new(|fut| {
            tokio::spawn(fut);
        }))
        .build();
    Ok(swarm)
}

// TODO: It would be nicer to have a struct with functions but the high level code is highly coupled with the API of libp2p's Swarm

// pub struct MyBehaviourSwarm {
//     swarm: Swarm<MyBehaviour>,
// }

// impl MyBehaviourSwarm {
//     pub async fn new(
//         topic: Topic,
//         transport: TcpTokioTransport,
//         peer_id: PeerId,
//     ) -> Result<MyBehaviourSwarm, ()> {
//         let mdns = Mdns::new(Default::default()).await.unwrap();
//         let mut behaviour = MyBehaviour::new(Floodsub::new(peer_id.clone()), mdns);
//         behaviour.floodsub().subscribe(topic.clone());
//         let swarm = SwarmBuilder::new(transport, behaviour, peer_id)
//             // We want the connection background tasks to be spawned
//             // onto the tokio runtime.
//             .executor(Box::new(|fut| {
//                 tokio::spawn(fut);
//             }))
//             .build();
//         Ok(MyBehaviourSwarm { swarm })
//     }
// }
