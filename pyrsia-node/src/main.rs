extern crate async_std;
extern crate clap;
use async_std::task;
use clap::{App, Arg, ArgMatches};
use futures::StreamExt;
use libp2p::core::muxing::StreamMuxerBox;
use libp2p::core::transport::Boxed;
use libp2p::kad::record::store::MemoryStore;
use libp2p::kad::{GetClosestPeersError, Kademlia, KademliaConfig, KademliaEvent, QueryResult};
use libp2p::{
    development_transport,
    identity::Keypair,
    swarm::{Swarm, SwarmEvent},
    Multiaddr, PeerId,
};
use std::{env, error::Error, str::FromStr, time::Duration};

fn join_authors(authors: Vec<&'static str>) -> String {
    return authors.join(", ");
}

#[async_std::main]
async fn main() -> Result<(), Box<dyn Error>> {
    env_logger::init();

    let boot_nodes: Vec<&'static str> = vec![
        "QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN",
        "QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa",
        "QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb",
        "QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt",
    ];

    // Create a random key for ourselves.
    let local_key: Keypair = Keypair::generate_ed25519();
    let local_peer_id: PeerId = PeerId::from(local_key.public());

    // Set up a an encrypted DNS-enabled TCP Transport over the Mplex protocol
    let transport: Boxed<(PeerId, StreamMuxerBox)> = development_transport(local_key).await?;

    let mut authors: Vec<&'static str> = Vec::new();
    authors.push("Joeri Sykora <joeri@sertik.net>");
    authors.push("Elliott Frisch <elliottf@jfrog.com>");
    let matches: ArgMatches = App::new("Pyrsia Node")
        .version("0.1.0")
        .author(&*join_authors(authors))
        .about("Application to connect to and participate in the Pyrsia network")
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .takes_value(false)
                .required(false)
                .multiple(true)
                .help("Enables verbose output"),
        )
        .get_matches();
    let verbosity: u64 = matches.occurrences_of("verbose");

    println!("Pyrsia Node is now running!");
    if verbosity > 0 {
        println!("Verbosity Level: {}", verbosity.to_string())
    }
    // Create a swarm to manage peers and events.
    let mut swarm: Swarm<Kademlia<MemoryStore>> = {
        // Create a Kademlia behaviour.
        let mut cfg: KademliaConfig = KademliaConfig::default();
        cfg.set_query_timeout(Duration::from_secs(5 * 60));
        let store: MemoryStore = MemoryStore::new(local_peer_id);
        let mut behaviour: Kademlia<MemoryStore> = Kademlia::with_config(local_peer_id, store, cfg);

        // Add the bootnodes to the local routing table. `libp2p-dns` built
        // into the `transport` resolves the `dnsaddr` when Kademlia tries
        // to dial these nodes.
        let bootaddr: Multiaddr = Multiaddr::from_str("/dnsaddr/bootstrap.libp2p.io")?;
        for peer in &boot_nodes {
            behaviour.add_address(&PeerId::from_str(peer)?, bootaddr.clone());
        }

        Swarm::new(transport, behaviour, local_peer_id)
    };

    // Order Kademlia to search for a peer.
    let to_search: PeerId = if let Some(peer_id) = env::args().nth(1) {
        peer_id.parse()?
    } else {
        Keypair::generate_ed25519().public().into()
    };

    println!("Searching for the closest peers to {:?}", to_search);
    swarm.behaviour_mut().get_closest_peers(to_search);

    // Kick it off!
    task::block_on(async move {
        loop {
            let event: SwarmEvent<KademliaEvent, std::io::Error> = swarm.select_next_some().await;
            if let SwarmEvent::Behaviour(KademliaEvent::OutboundQueryCompleted {
                result: QueryResult::GetClosestPeers(result),
                ..
            }) = event
            {
                match result {
                    Ok(ok) => {
                        if !ok.peers.is_empty() {
                            println!("Query finished with closest peers: {:#?}", ok.peers)
                        } else {
                            // The example is considered failed as there
                            // should always be at least 1 reachable peer.
                            println!("Query finished with no closest peers.")
                        }
                    }
                    Err(GetClosestPeersError::Timeout { peers, .. }) => {
                        if !peers.is_empty() {
                            println!("Query timed out with closest peers: {:#?}", peers)
                        } else {
                            // The example is considered failed as there
                            // should always be at least 1 reachable peer.
                            println!("Query timed out with no closest peers.");
                        }
                    }
                };

                break;
            }
        }

        Ok(())
    })
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_add() {
        assert_eq!("a, b, c", join_authors(Vec::from(["a", "b", "c"])));
    }
}
