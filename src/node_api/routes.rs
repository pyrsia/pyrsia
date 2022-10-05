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
use crate::network::client::Client;
use crate::node_api::model::cli::{RequestAddAuthorizedNode, RequestDockerLog, RequestMavenLog};
use crate::transparency_log::log::TransparencyLogService;
use warp::Filter;

pub fn make_node_routes(
    artifact_service: ArtifactService,
    p2p_client: Client,
    transparency_log_service: TransparencyLogService,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let artifact_service_filter = warp::any().map(move || artifact_service.clone());
    let p2p_client_filter = warp::any().map(move || p2p_client.clone());
    let transparency_log_service_filter = warp::any().map(move || transparency_log_service.clone());

    let add_authorized_node = warp::path!("authorized_node")
        .and(warp::post())
        .and(warp::path::end())
        .and(warp::body::content_length_limit(1024 * 8))
        .and(warp::body::json::<RequestAddAuthorizedNode>())
        .and(transparency_log_service_filter.clone())
        .and_then(handle_add_authorized_node);

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
        .and(artifact_service_filter)
        .and_then(handle_build_maven);

    let peers = warp::path!("peers")
        .and(warp::get())
        .and(warp::path::end())
        .and(p2p_client_filter.clone())
        .and_then(handle_get_peers);

    let status = warp::path!("status")
        .and(warp::get())
        .and(warp::path::end())
        .and(p2p_client_filter)
        .and_then(handle_get_status);

    let inspect_docker = warp::path!("inspect" / "docker")
        .and(warp::post())
        .and(warp::path::end())
        .and(warp::body::content_length_limit(1024 * 8))
        .and(warp::body::json::<RequestDockerLog>())
        .and(transparency_log_service_filter.clone())
        .and_then(handle_inspect_log_docker);

    let inspect_maven = warp::path!("inspect" / "maven")
        .and(warp::post())
        .and(warp::path::end())
        .and(warp::body::content_length_limit(1024 * 8))
        .and(warp::body::json::<RequestMavenLog>())
        .and(transparency_log_service_filter)
        .and_then(handle_inspect_log_maven);

    warp::any().and(
        add_authorized_node
            .or(build_docker)
            .or(build_maven)
            .or(peers)
            .or(status)
            .or(inspect_docker)
            .or(inspect_maven),
    )
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use crate::blockchain_service::service::BlockchainService;
    use crate::build_service::event::{BuildEvent, BuildEventClient};
    use crate::network::client::command::Command;
    use crate::network::client::Client;
    use crate::node_api::model::cli::Status;
    use crate::util::test_util;
    use libp2p::identity::Keypair;
    use std::collections::HashSet;
    use std::path::Path;
    use std::str;
    use std::sync::Arc;
    use tokio::sync::{mpsc, Mutex};

    fn create_p2p_client(local_keypair: &Keypair) -> (mpsc::Receiver<Command>, Client) {
        let (command_sender, command_receiver) = mpsc::channel(1);
        let p2p_client = Client {
            sender: command_sender,
            local_peer_id: local_keypair.public().to_peer_id(),
        };

        (command_receiver, p2p_client)
    }

    async fn create_blockchain_service(
        local_keypair: &Keypair,
        p2p_client: Client,
        blockchain_path: impl AsRef<Path>,
    ) -> BlockchainService {
        let ed25519_keypair = match local_keypair {
            libp2p::identity::Keypair::Ed25519(ref v) => v,
            _ => {
                panic!("Keypair Format Error");
            }
        };

        BlockchainService::init_first_blockchain_node(
            ed25519_keypair,
            ed25519_keypair,
            p2p_client,
            blockchain_path,
        )
        .await
        .expect("Creating BlockchainService failed")
    }

    fn create_artifact_service(
        artifact_path: impl AsRef<Path>,
        transparency_log_service: TransparencyLogService,
        p2p_client: Client,
    ) -> (mpsc::Receiver<BuildEvent>, ArtifactService) {
        let (build_event_sender, build_event_receiver) = mpsc::channel(1);
        let build_event_client = BuildEventClient::new(build_event_sender);

        let artifact_service = ArtifactService::new(
            &artifact_path,
            transparency_log_service,
            build_event_client,
            p2p_client,
        )
        .expect("Creating ArtifactService failed");

        (build_event_receiver, artifact_service)
    }

    async fn create_transparency_log_service(
        artifact_path: impl AsRef<Path>,
        local_keypair: Keypair,
        p2p_client: Client,
    ) -> TransparencyLogService {
        let blockchain_service =
            create_blockchain_service(&local_keypair, p2p_client, &artifact_path).await;

        TransparencyLogService::new(artifact_path, Arc::new(Mutex::new(blockchain_service)))
            .expect("Creating ArtifactService failed")
    }

    #[tokio::test]
    async fn node_routes_add_authorized_node() {
        let tmp_dir = test_util::tests::setup();

        let local_keypair = Keypair::generate_ed25519();
        let (mut command_receiver, p2p_client) = create_p2p_client(&local_keypair);
        let transparency_log_service =
            create_transparency_log_service(&tmp_dir, local_keypair.clone(), p2p_client.clone())
                .await;

        let (_build_event_receiver, artifact_service) = create_artifact_service(
            &tmp_dir,
            transparency_log_service.clone(),
            p2p_client.clone(),
        );

        tokio::spawn(async move {
            loop {
                match command_receiver.recv().await {
                    Some(Command::ListPeers { sender, .. }) => {
                        let _ = sender.send(HashSet::new());
                    }
                    _ => panic!("Command must match Command::ListPeers"),
                }
            }
        });

        let filter = make_node_routes(artifact_service, p2p_client, transparency_log_service);
        let request = RequestAddAuthorizedNode {
            peer_id: local_keypair.public().to_peer_id().to_string(),
        };
        let response = warp::test::request()
            .method("POST")
            .path("/authorized_node")
            .json(&request)
            .reply(&filter)
            .await;

        assert_eq!(response.status(), 201);
        assert_eq!(response.body(), "");

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn node_routes_build_docker() {
        let tmp_dir = test_util::tests::setup();

        let local_keypair = Keypair::generate_ed25519();
        let (mut command_receiver, p2p_client) = create_p2p_client(&local_keypair);
        let transparency_log_service =
            create_transparency_log_service(&tmp_dir, local_keypair.clone(), p2p_client.clone())
                .await;

        let (mut build_event_receiver, artifact_service) = create_artifact_service(
            &tmp_dir,
            transparency_log_service.clone(),
            p2p_client.clone(),
        );

        tokio::spawn(async move {
            loop {
                match command_receiver.recv().await {
                    Some(Command::ListPeers { sender, .. }) => {
                        let _ = sender.send(HashSet::new());
                    }
                    _ => panic!("Command must match Command::ListPeers"),
                }
            }
        });

        transparency_log_service
            .add_authorized_node(local_keypair.public().to_peer_id())
            .await
            .expect("Error adding authorized node");

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

        let filter = make_node_routes(artifact_service, p2p_client, transparency_log_service);
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

        let local_keypair = Keypair::generate_ed25519();
        let (mut command_receiver, p2p_client) = create_p2p_client(&local_keypair);
        let transparency_log_service =
            create_transparency_log_service(&tmp_dir, local_keypair.clone(), p2p_client.clone())
                .await;

        let (mut build_event_receiver, artifact_service) = create_artifact_service(
            &tmp_dir,
            transparency_log_service.clone(),
            p2p_client.clone(),
        );

        tokio::spawn(async move {
            loop {
                match command_receiver.recv().await {
                    Some(Command::ListPeers { sender, .. }) => {
                        let _ = sender.send(HashSet::new());
                    }
                    _ => panic!("Command must match Command::ListPeers"),
                }
            }
        });

        transparency_log_service
            .add_authorized_node(local_keypair.public().to_peer_id())
            .await
            .expect("Error adding authorized node");

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

        let filter = make_node_routes(artifact_service, p2p_client, transparency_log_service);
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

        let local_keypair = Keypair::generate_ed25519();
        let (mut command_receiver, p2p_client) = create_p2p_client(&local_keypair);
        let transparency_log_service =
            create_transparency_log_service(&tmp_dir, local_keypair, p2p_client.clone()).await;
        let (_build_event_receiver, artifact_service) = create_artifact_service(
            &tmp_dir,
            transparency_log_service.clone(),
            p2p_client.clone(),
        );

        tokio::spawn(async move {
            loop {
                match command_receiver.recv().await {
                    Some(Command::ListPeers { sender, .. }) => {
                        let mut set = HashSet::new();
                        set.insert(p2p_client.local_peer_id);
                        let _ = sender.send(set);
                    }
                    _ => panic!("Command must match Command::ListPeers"),
                }
            }
        });

        let filter = make_node_routes(
            artifact_service,
            p2p_client.clone(),
            transparency_log_service,
        );
        let response = warp::test::request().path("/peers").reply(&filter).await;

        let expected_body =
            bytes::Bytes::from(serde_json::to_string(&vec![p2p_client.local_peer_id]).unwrap());

        assert_eq!(response.status(), 200);
        assert_eq!(expected_body, str::from_utf8(response.body()).unwrap());

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn node_routes_status() {
        let tmp_dir = test_util::tests::setup();

        let local_keypair = Keypair::generate_ed25519();
        let (mut command_receiver, p2p_client) = create_p2p_client(&local_keypair);
        let transparency_log_service =
            create_transparency_log_service(&tmp_dir, local_keypair, p2p_client.clone()).await;
        let (_build_event_receiver, artifact_service) = create_artifact_service(
            &tmp_dir,
            transparency_log_service.clone(),
            p2p_client.clone(),
        );

        tokio::spawn(async move {
            loop {
                match command_receiver.recv().await {
                    Some(Command::ListPeers { sender, .. }) => {
                        let mut set = HashSet::new();
                        set.insert(p2p_client.local_peer_id);
                        let _ = sender.send(set);
                    }
                    Some(Command::Status { sender, .. }) => {
                        let status = Status {
                            peers_count: 0,
                            peer_addrs: Vec::new(),
                            peer_id: p2p_client.local_peer_id.to_string(),
                        };

                        let _ = sender.send(status);
                    }
                    _ => panic!("Command must match Command::ListPeers or Command::Status"),
                }
            }
        });

        let filter = make_node_routes(
            artifact_service,
            p2p_client.clone(),
            transparency_log_service,
        );
        let response = warp::test::request().path("/status").reply(&filter).await;

        let expected_status = Status {
            peers_count: 0,
            peer_id: p2p_client.local_peer_id.to_string(),
            peer_addrs: Vec::new(),
        };

        let expected_body = bytes::Bytes::from(serde_json::to_string(&expected_status).unwrap());

        assert_eq!(response.status(), 200);
        assert_eq!(expected_body, str::from_utf8(response.body()).unwrap());

        test_util::tests::teardown(tmp_dir);
    }
}
