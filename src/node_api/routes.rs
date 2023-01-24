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
use crate::node_api::model::cli::{
    RequestAddAuthorizedNode, RequestBuildStatus, RequestDockerLog, RequestMavenLog,
};
use warp::Filter;

pub fn make_node_routes(
    artifact_service: ArtifactService,
    p2p_client: Client,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
    let artifact_service_filter = warp::any().map(move || artifact_service.clone());
    let p2p_client_filter = warp::any().map(move || p2p_client.clone());

    let add_authorized_node = warp::path!("authorized_node")
        .and(warp::post())
        .and(warp::path::end())
        .and(warp::body::content_length_limit(1024 * 8))
        .and(warp::body::json::<RequestAddAuthorizedNode>())
        .and(artifact_service_filter.clone())
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
        .and(artifact_service_filter.clone())
        .and_then(handle_build_maven);

    let build_status = warp::path!("build" / "status")
        .and(warp::post())
        .and(warp::path::end())
        .and(warp::body::content_length_limit(1024 * 8))
        .and(warp::body::json::<RequestBuildStatus>())
        .and(artifact_service_filter.clone())
        .and_then(handle_build_status);

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
        .and(artifact_service_filter.clone())
        .and_then(handle_inspect_log_docker);

    let inspect_maven = warp::path!("inspect" / "maven")
        .and(warp::post())
        .and(warp::path::end())
        .and(warp::body::content_length_limit(1024 * 8))
        .and(warp::body::json::<RequestMavenLog>())
        .and(artifact_service_filter)
        .and_then(handle_inspect_log_maven);

    warp::any().and(
        add_authorized_node
            .or(build_docker)
            .or(build_maven)
            .or(peers)
            .or(status)
            .or(inspect_docker)
            .or(inspect_maven)
            .or(build_status),
    )
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use crate::artifact_service::model::PackageType;
    use crate::blockchain_service::event::BlockchainEvent;
    use crate::build_service::event::BuildEvent;
    use crate::network::client::command::Command;
    use crate::node_api::model::cli::{Status, TransparencyLogOutputParams};
    use crate::transparency_log::log::{
        AddArtifactRequest, TransparencyLog, TransparencyLogService,
    };
    use crate::util::test_util;
    use csv;
    use httptest::http;
    use std::collections::HashSet;
    use std::future::Future;
    use std::str;

    #[tokio::test]
    async fn node_routes_add_authorized_node() {
        let tmp_dir = test_util::tests::setup();

        let (p2p_client, mut p2p_command_receiver) = test_util::tests::create_p2p_client();
        let (artifact_service, mut blockchain_event_receiver, ..) =
            test_util::tests::create_artifact_service_with_p2p_client(&tmp_dir, p2p_client.clone());

        tokio::spawn(async move {
            loop {
                match blockchain_event_receiver.recv().await {
                    Some(BlockchainEvent::AddBlock { sender, .. }) => {
                        let _ = sender.send(Ok(()));
                    }
                    _ => panic!("BlockchainEvent must match BlockchainEvent::AddBlock"),
                }
            }
        });

        tokio::spawn(async move {
            loop {
                match p2p_command_receiver.recv().await {
                    Some(Command::ListPeers { sender, .. }) => {
                        let _ = sender.send(HashSet::new());
                    }
                    _ => panic!("Command must match Command::ListPeers"),
                }
            }
        });

        let filter = make_node_routes(artifact_service, p2p_client.clone());
        let request = RequestAddAuthorizedNode {
            peer_id: p2p_client.local_peer_id.to_string(),
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

        let (p2p_client, mut p2p_command_receiver) = test_util::tests::create_p2p_client();
        let (artifact_service, mut blockchain_event_receiver, mut build_event_receiver) =
            test_util::tests::create_artifact_service_with_p2p_client(&tmp_dir, p2p_client.clone());

        tokio::spawn(async move {
            loop {
                match blockchain_event_receiver.recv().await {
                    Some(BlockchainEvent::AddBlock { sender, .. }) => {
                        let _ = sender.send(Ok(()));
                    }
                    _ => panic!("BlockchainEvent must match BlockchainEvent::AddBlock"),
                }
            }
        });

        let build_id = uuid::Uuid::new_v4();
        tokio::spawn(async move {
            loop {
                match build_event_receiver.recv().await {
                    Some(BuildEvent::Start { sender, .. }) => {
                        let _ = sender.send(Ok(build_id.to_string()));
                    }
                    _ => {
                        panic!("BuildEvent must match BuildEvent::Start")
                    }
                }
            }
        });

        tokio::spawn(async move {
            loop {
                match p2p_command_receiver.recv().await {
                    Some(Command::ListPeers { sender, .. }) => {
                        let _ = sender.send(HashSet::new());
                    }
                    _ => panic!("Command must match Command::ListPeers"),
                }
            }
        });

        artifact_service
            .transparency_log_service
            .add_authorized_node(p2p_client.local_peer_id)
            .await
            .expect("Error adding authorized node");

        let filter = make_node_routes(artifact_service, p2p_client);
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

        let (p2p_client, mut p2p_command_receiver) = test_util::tests::create_p2p_client();
        let (artifact_service, mut blockchain_event_receiver, mut build_event_receiver) =
            test_util::tests::create_artifact_service_with_p2p_client(&tmp_dir, p2p_client.clone());

        tokio::spawn(async move {
            loop {
                match blockchain_event_receiver.recv().await {
                    Some(BlockchainEvent::AddBlock { sender, .. }) => {
                        let _ = sender.send(Ok(()));
                    }
                    _ => panic!("BlockchainEvent must match BlockchainEvent::AddBlock"),
                }
            }
        });

        let build_id = uuid::Uuid::new_v4();
        tokio::spawn(async move {
            loop {
                match build_event_receiver.recv().await {
                    Some(BuildEvent::Start { sender, .. }) => {
                        let _ = sender.send(Ok(build_id.to_string()));
                    }
                    _ => {
                        panic!("BuildEvent must match BuildEvent::Start")
                    }
                }
            }
        });

        tokio::spawn(async move {
            loop {
                match p2p_command_receiver.recv().await {
                    Some(Command::ListPeers { sender, .. }) => {
                        let _ = sender.send(HashSet::new());
                    }
                    _ => panic!("Command must match Command::ListPeers"),
                }
            }
        });

        artifact_service
            .transparency_log_service
            .add_authorized_node(p2p_client.local_peer_id)
            .await
            .expect("Error adding authorized node");

        let filter = make_node_routes(artifact_service, p2p_client);
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

        let (p2p_client, mut p2p_command_receiver) = test_util::tests::create_p2p_client();
        let (artifact_service, ..) =
            test_util::tests::create_artifact_service_with_p2p_client(&tmp_dir, p2p_client.clone());

        tokio::spawn(async move {
            loop {
                match p2p_command_receiver.recv().await {
                    Some(Command::ListPeers { sender, .. }) => {
                        let mut set = HashSet::new();
                        set.insert(p2p_client.local_peer_id);
                        let _ = sender.send(set);
                    }
                    _ => panic!("Command must match Command::ListPeers"),
                }
            }
        });

        let filter = make_node_routes(artifact_service, p2p_client.clone());
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

        let (p2p_client, mut p2p_command_receiver) = test_util::tests::create_p2p_client();
        let (artifact_service, ..) =
            test_util::tests::create_artifact_service_with_p2p_client(&tmp_dir, p2p_client.clone());

        let local_peer_id = p2p_client.local_peer_id;
        tokio::spawn(async move {
            loop {
                match p2p_command_receiver.recv().await {
                    Some(Command::ListPeers { sender, .. }) => {
                        let mut set = HashSet::new();
                        set.insert(local_peer_id);
                        let _ = sender.send(set);
                    }
                    Some(Command::Status { sender, .. }) => {
                        let status = Status {
                            peers_count: 0,
                            peer_addrs: Vec::new(),
                            peer_id: local_peer_id.to_string(),
                        };

                        let _ = sender.send(status);
                    }
                    _ => panic!("Command must match Command::ListPeers or Command::Status"),
                }
            }
        });

        let filter = make_node_routes(artifact_service, p2p_client.clone());
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

    // Inspect Transparency Log Tests

    #[tokio::test]
    async fn inspect_log_docker_json() {
        setup_and_execute(|ctx| async {
            let ps_id = "library/artipie:0.0.7";
            let transparency_log = add_artifact(&ctx.log, PackageType::Docker, ps_id);
            let request = RequestDockerLog {
                image: ps_id.to_string(),
                output_params: Default::default(),
            };

            let filter = ctx.create_route();

            let response = warp::test::request()
                .method("POST")
                .path("/inspect/docker")
                .json(&request)
                .reply(&filter)
                .await;

            assert_response_json(response, transparency_log);
        })
        .await;
    }

    #[tokio::test]
    async fn inspect_log_docker_csv() {
        setup_and_execute(|ctx| async {
            let ps_id = "library/artipie:0.0.7";
            let transparency_log = add_artifact(&ctx.log, PackageType::Docker, ps_id);
            let request = RequestDockerLog {
                image: ps_id.to_string(),
                output_params: Some(TransparencyLogOutputParams {
                    format: Some(ContentType::CSV),
                }),
            };
            let filter = ctx.create_route();
            let response = warp::test::request()
                .method("POST")
                .path("/inspect/docker")
                .json(&request)
                .reply(&filter)
                .await;

            assert_response_csv(response, transparency_log);
        })
        .await;
    }

    #[tokio::test]
    async fn inspect_log_maven_json() {
        setup_and_execute(|ctx| async {
            let ps_id = "pyrsia:adapter:0.1";
            let transparency_log = add_artifact(&ctx.log, PackageType::Maven2, ps_id);
            let request = RequestMavenLog {
                gav: ps_id.to_string(),
                output_params: None,
            };
            let filter = ctx.create_route();
            let response = warp::test::request()
                .method("POST")
                .path("/inspect/maven")
                .json(&request)
                .reply(&filter)
                .await;

            assert_response_json(response, transparency_log);
        })
        .await;
    }

    #[tokio::test]
    async fn inspect_log_maven_csv() {
        setup_and_execute(|ctx| async {
            let ps_id = "pyrsia:adapter:0.1";
            let transparency_log = add_artifact(&ctx.log, PackageType::Maven2, ps_id);
            let request = RequestMavenLog {
                gav: ps_id.to_string(),
                output_params: Some(TransparencyLogOutputParams {
                    format: Some(ContentType::CSV),
                }),
            };
            let filter = ctx.create_route();
            let response = warp::test::request()
                .method("POST")
                .path("/inspect/maven")
                .json(&request)
                .reply(&filter)
                .await;

            assert_response_csv(response, transparency_log);
        })
        .await;
    }

    fn assert_response_csv(
        response: http::response::Response<bytes::Bytes>,
        transparency_log: TransparencyLog,
    ) {
        let mut writer = csv::Writer::from_writer(vec![]);
        writer.serialize(transparency_log).unwrap();
        let expected = writer.into_inner().unwrap();
        assert_response(response, "text/csv", expected.as_slice());
    }

    fn assert_response_json(
        response: http::response::Response<bytes::Bytes>,
        transparency_log: TransparencyLog,
    ) {
        let res = vec![transparency_log];
        let expected = serde_json::to_string(&res).unwrap();
        assert_response(response, "application/json", expected.as_bytes());
    }

    fn assert_response(
        response: http::response::Response<bytes::Bytes>,
        content_type: &str,
        body: &[u8],
    ) {
        assert_eq!(response.status(), 200);
        assert_eq!(
            response.headers().get("content-type").unwrap(),
            content_type
        );

        let actual_body = response.body();
        assert_eq!(actual_body, body);

        let actual_content_length = response
            .headers()
            .get("Content-Length")
            .expect("Context-Length header should be defined")
            .to_str()
            .unwrap();
        assert_eq!(actual_content_length, body.len().to_string());
    }

    fn add_artifact(
        transparency_log_service: &TransparencyLogService,
        package_type: PackageType,
        ps_id: &str,
    ) -> TransparencyLog {
        let add_art_req = AddArtifactRequest {
            package_type,
            package_specific_id: ps_id.to_string(),
            num_artifacts: 5,
            package_specific_artifact_id: ps_id.to_string(),
            artifact_hash: "test_hash".to_string(),
        };
        let res = TransparencyLog::from(add_art_req);
        transparency_log_service
            .write_transparency_log(&res)
            .unwrap();

        res
    }

    async fn setup_and_execute<P, F>(op: P)
    where
        P: FnOnce(TestContext) -> F,
        F: Future<Output = ()>,
    {
        let tmp_dir = test_util::tests::setup();
        let (p2p_client, _) = test_util::tests::create_p2p_client();
        let (artifact_service, ..) =
            test_util::tests::create_artifact_service_with_p2p_client(&tmp_dir, p2p_client.clone());
        let (log, _) = test_util::tests::create_transparency_log_service(&tmp_dir);

        op(TestContext {
            artifact_service,
            p2p_client,
            log,
        })
        .await;

        test_util::tests::teardown(tmp_dir);
    }

    struct TestContext {
        artifact_service: ArtifactService,
        p2p_client: Client,
        log: TransparencyLogService,
    }

    impl TestContext {
        fn create_route(
            self,
        ) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
            make_node_routes(self.artifact_service, self.p2p_client)
        }
    }
}
