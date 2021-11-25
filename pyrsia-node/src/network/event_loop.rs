use super::swarm::MyBehaviourSwarm;
use futures::prelude::*;
use futures::StreamExt;
use libp2p::swarm::{ProtocolsHandlerUpgrErr, SwarmBuilder, SwarmEvent};
use std::io::Error;

pub struct EventLoop {
    swarm: MyBehaviourSwarm,
}

impl EventLoop {
    fn new(swarm: MyBehaviourSwarm) -> Self {
        Self { swarm }
    }

    pub async fn run(mut self) {
        loop {
            tokio::select! {
            event = self.swarm.select_next_some() =>
            self.handle_event(event).await  ,
            }
        }
    }

    async fn handle_event<Error>(&mut self, event: libp2p::swarm::SwarmEvent<libp2p::kad::KademliaEvent, Error>) {
        match event {
            SwarmEvent::Behaviour(libp2p::kad::KademliaEvent::OutboundQueryCompleted {
                id,
                result:
                    libp2p::kad::QueryResult::GetProviders(Ok(libp2p::kad::GetProvidersOk {
                        providers,
                        ..
                    })),
                ..
            }) => {
                // let _ = pending_get_providers
                //     .remove(&id)
                //     .expect("Completed query to be previously pending.");
                println!("Obtained providers");
            }
        }
    }
}
