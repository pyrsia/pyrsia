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

use super::handlers::swarm::*;
use super::model::cli::{RequestDockerBuild, RequestMavenBuild};
use crate::artifact_service::service::ArtifactService;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::Filter;

pub fn make_node_routes(
    artifact_service: Arc<Mutex<ArtifactService>>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let artifact_service_filter = warp::any().map(move || artifact_service.clone());

    let build_docker = warp::path!("build" / "docker")
        .and(warp::post())
        .and(warp::path::end())
        .and(warp::body::content_length_limit(1024 * 8))
        .and(warp::body::json::<RequestDockerBuild>())
        .and(artifact_service_filter.clone())
        .and_then(handle_build_docker);

    let build_maven = warp::path!("build" / "maven")
        .and(warp::post())
        .and(warp::path::end())
        .and(warp::body::content_length_limit(1024 * 8))
        .and(warp::body::json::<RequestMavenBuild>())
        .and(artifact_service_filter.clone())
        .and_then(handle_build_maven);

    let peers = warp::path!("peers")
        .and(warp::get())
        .and(warp::path::end())
        .and(artifact_service_filter.clone())
        .and_then(handle_get_peers);

    let status = warp::path!("status")
        .and(warp::get())
        .and(warp::path::end())
        .and(artifact_service_filter)
        .and_then(handle_get_status);

    warp::any().and(build_docker.or(build_maven).or(peers).or(status))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build_service::event::{BuildEvent, BuildEventClient};
    use crate::network::client::command::Command;
    use crate::network::client::Client;
    use crate::node_api::model::cli::Status;
    use crate::util::test_util;
    use libp2p::identity::Keypair;
    use std::collections::HashSet;
    use std::str;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn node_routes_build_docker() {
        let tmp_dir = test_util::tests::setup();

        let (command_sender, _command_receiver) = mpsc::channel(1);
        let p2p_client = Client {
            sender: command_sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
        };

        let (build_event_sender, mut build_event_receiver) = mpsc::channel(1);
        let build_event_client = BuildEventClient::new(build_event_sender);
        let artifact_service = ArtifactService::new(&tmp_dir, build_event_client, p2p_client)
            .expect("Creating ArtifactService failed");

        let build_id = uuid::Uuid::new_v4();
        tokio::spawn(async move {
            loop {
                match build_event_receiver.recv().await {
                    Some(BuildEvent::Start { sender, .. }) => {
                        let _ = sender.send(Ok(build_id.to_string()));
                    }
                    _ => panic!("BuildEvent must match BuildEvent::Start"),
                }
            }
        });

        let filter = make_node_routes(Arc::new(Mutex::new(artifact_service)));
        let request = RequestDockerBuild {
            image: "alpine:3.15.2".to_owned(),
        };
        let response = warp::test::request()
            .method("POST")
            .path("/build/docker")
            .json(&request)
            .reply(&filter)
            .await;

        assert_eq!(response.status(), 200);

        let build_id_result: String = serde_json::from_slice(response.body()).unwrap();
        assert_eq!(build_id_result, build_id.to_string());

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn node_routes_build_maven() {
        let tmp_dir = test_util::tests::setup();

        let (command_sender, _command_receiver) = mpsc::channel(1);
        let p2p_client = Client {
            sender: command_sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
        };

        let (build_event_sender, mut build_event_receiver) = mpsc::channel(1);
        let build_event_client = BuildEventClient::new(build_event_sender);
        let artifact_service = ArtifactService::new(&tmp_dir, build_event_client, p2p_client)
            .expect("Creating ArtifactService failed");

        let build_id = uuid::Uuid::new_v4();
        tokio::spawn(async move {
            loop {
                match build_event_receiver.recv().await {
                    Some(BuildEvent::Start { sender, .. }) => {
                        let _ = sender.send(Ok(build_id.to_string()));
                    }
                    _ => panic!("BuildEvent must match BuildEvent::Start"),
                }
            }
        });

        let filter = make_node_routes(Arc::new(Mutex::new(artifact_service)));
        let request = RequestMavenBuild {
            gav: "commons-codec:commons-codec:1.15".to_owned(),
        };
        let response = warp::test::request()
            .method("POST")
            .path("/build/maven")
            .json(&request)
            .reply(&filter)
            .await;

        assert_eq!(response.status(), 200);

        let build_id_result: String = serde_json::from_slice(response.body()).unwrap();
        assert_eq!(build_id_result, build_id.to_string());

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn node_routes_peers() {
        let tmp_dir = test_util::tests::setup();

        let (command_sender, mut command_receiver) = mpsc::channel(1);
        let local_peer_id = Keypair::generate_ed25519().public().to_peer_id();
        let p2p_client = Client {
            sender: command_sender,
            local_peer_id,
        };

        tokio::spawn(async move {
            loop {
                match command_receiver.recv().await {
                    Some(Command::ListPeers { sender, .. }) => {
                        let mut set = HashSet::new();
                        set.insert(local_peer_id);
                        let _ = sender.send(set);
                    }
                    _ => panic!("Command must match Command::ListPeers"),
                }
            }
        });

        let (build_event_sender, _build_event_receiver) = mpsc::channel(1);
        let build_event_client = BuildEventClient::new(build_event_sender);
        let artifact_service = ArtifactService::new(&tmp_dir, build_event_client, p2p_client)
            .expect("Creating ArtifactService failed");

        let filter = make_node_routes(Arc::new(Mutex::new(artifact_service)));
        let response = warp::test::request().path("/peers").reply(&filter).await;

        let expected_body =
            bytes::Bytes::from(serde_json::to_string(&vec![local_peer_id]).unwrap());

        assert_eq!(response.status(), 200);
        assert_eq!(expected_body, str::from_utf8(response.body()).unwrap());

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn node_routes_status() {
        let tmp_dir = test_util::tests::setup();

        let (command_sender, mut command_receiver) = mpsc::channel(1);
        let local_peer_id = Keypair::generate_ed25519().public().to_peer_id();
        let p2p_client = Client {
            sender: command_sender,
            local_peer_id,
        };

        tokio::spawn(async move {
            loop {
                match command_receiver.recv().await {
                    Some(Command::ListPeers { sender, .. }) => {
                        let mut set = HashSet::new();
                        set.insert(local_peer_id);
                        let _ = sender.send(set);
                    }
                    _ => panic!("Command must match Command::ListPeers"),
                }
            }
        });

        let (build_event_sender, _build_event_receiver) = mpsc::channel(1);
        let build_event_client = BuildEventClient::new(build_event_sender);
        let artifact_service = ArtifactService::new(&tmp_dir, build_event_client, p2p_client)
            .expect("Creating ArtifactService failed");

        let filter = make_node_routes(Arc::new(Mutex::new(artifact_service)));
        let response = warp::test::request().path("/status").reply(&filter).await;

        let expected_status = Status {
            peers_count: 1,
            peer_id: local_peer_id.to_string(),
        };

        let expected_body = bytes::Bytes::from(serde_json::to_string(&expected_status).unwrap());

        assert_eq!(response.status(), 200);
        assert_eq!(expected_body, str::from_utf8(response.body()).unwrap());

        test_util::tests::teardown(tmp_dir);
    }
}
