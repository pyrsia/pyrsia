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
use crate::build_service::service::BuildService;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::Filter;

pub fn make_node_routes(
    artifact_service: Arc<Mutex<ArtifactService>>,
    build_service: Arc<Mutex<BuildService>>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let artifact_service_filter = warp::any().map(move || artifact_service.clone());
    let build_service_filter = warp::any().map(move || build_service.clone());

    let build_docker = warp::path!("build" / "docker")
        .and(warp::post())
        .and(warp::path::end())
        .and(warp::body::content_length_limit(1024 * 8))
        .and(warp::body::json::<RequestDockerBuild>())
        .and(build_service_filter.clone())
        .and_then(handle_build_docker);

    let build_maven = warp::path!("build" / "maven")
        .and(warp::post())
        .and(warp::path::end())
        .and(warp::body::content_length_limit(1024 * 8))
        .and(warp::body::json::<RequestMavenBuild>())
        .and(build_service_filter)
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
    use crate::artifact_service::model::PackageType;
    use crate::build_service::mapping::model::{MappingInfo, SourceRepository};
    use crate::build_service::model::{BuildInfo, BuildStatus};
    use crate::build_service::service::BuildService;
    use crate::network::client::command::Command;
    use crate::network::client::Client;
    use crate::node_api::model::cli::Status;
    use crate::util::test_util;
    use httptest::{matchers, responders, Expectation, Server};
    use libp2p::identity::Keypair;
    use std::collections::HashSet;
    use std::str;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn node_routes_build_docker() {
        let tmp_dir = test_util::tests::setup();

        let mapping_info = MappingInfo {
            package_type: PackageType::Docker,
            package_specific_id: "alpine:3.15.2".to_owned(),
            source_repository: None,
            build_spec_url: None,
        };

        let build_info = BuildInfo {
            id: uuid::Uuid::new_v4().to_string(),
            status: BuildStatus::Running,
        };

        let http_server = Server::run();
        http_server.expect(
            Expectation::matching(matchers::all_of!(
                matchers::request::method_path("PUT", "/build"),
                matchers::request::body(matchers::json_decoded(matchers::eq(serde_json::json!(
                    &mapping_info
                ))))
            ))
            .respond_with(responders::json_encoded(&build_info)),
        );

        let http_server_url = &http_server.url_str("/");

        let (command_sender, _command_receiver) = mpsc::channel(1);
        let p2p_client = Client {
            sender: command_sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
        };

        let (build_command_sender, _build_command_receiver) = mpsc::channel(1);
        let artifact_service =
            ArtifactService::new(&tmp_dir, build_command_sender.clone(), p2p_client)
                .expect("Creating ArtifactService failed");

        let build_service = BuildService::new(
            &tmp_dir,
            build_command_sender,
            http_server_url,
            http_server_url,
        )
        .expect("Creating BuildService failed");

        let filter = make_node_routes(
            Arc::new(Mutex::new(artifact_service)),
            Arc::new(Mutex::new(build_service)),
        );
        let request = RequestDockerBuild {
            manifest: "alpine:3.15.2".to_owned(),
        };
        let response = warp::test::request()
            .method("POST")
            .path("/build/docker")
            .json(&request)
            .reply(&filter)
            .await;

        assert_eq!(response.status(), 200);

        let build_info: BuildInfo = serde_json::from_slice(response.body()).unwrap();
        assert_eq!(build_info.status, BuildStatus::Running);

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn node_routes_build_maven() {
        let tmp_dir = test_util::tests::setup();

        let mapping_info = MappingInfo {
            package_type: PackageType::Maven2,
            package_specific_id: "commons-codec:commons-codec:1.15".to_owned(),
            source_repository: Some(SourceRepository::Git {
                url: "https://github.com/apache/commons-codec".to_owned(),
                tag: "rel/commons-codec-1.15".to_owned()
            }),
            build_spec_url: Some("https://raw.githubusercontent.com/pyrsia/pyrsia-mappings/main/Maven2/commons-codec/commons-codec/1.15/commons-codec-1.15.buildspec".to_owned()),
        };

        let build_info = BuildInfo {
            id: uuid::Uuid::new_v4().to_string(),
            status: BuildStatus::Running,
        };

        let http_server = Server::run();
        http_server.expect(
            Expectation::matching(matchers::request::method_path(
                "GET",
                "/Maven2/commons-codec/commons-codec/1.15/commons-codec-1.15.mapping",
            ))
            .respond_with(responders::json_encoded(&mapping_info)),
        );
        http_server.expect(
            Expectation::matching(matchers::all_of!(
                matchers::request::method_path("PUT", "/build"),
                matchers::request::body(matchers::json_decoded(matchers::eq(serde_json::json!(
                    &mapping_info
                ))))
            ))
            .respond_with(responders::json_encoded(&build_info)),
        );

        let http_server_url = &http_server.url_str("/");

        let (command_sender, _command_receiver) = mpsc::channel(1);
        let p2p_client = Client {
            sender: command_sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
        };

        let (build_command_sender, _build_command_receiver) = mpsc::channel(1);
        let artifact_service =
            ArtifactService::new(&tmp_dir, build_command_sender.clone(), p2p_client)
                .expect("Creating ArtifactService failed");

        let build_service = BuildService::new(
            &tmp_dir,
            build_command_sender,
            http_server_url,
            http_server_url,
        )
        .expect("Creating BuildService failed");

        let filter = make_node_routes(
            Arc::new(Mutex::new(artifact_service)),
            Arc::new(Mutex::new(build_service)),
        );
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

        let build_info: BuildInfo = serde_json::from_slice(response.body()).unwrap();
        assert_eq!(build_info.status, BuildStatus::Running);

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

        let (build_command_sender, _build_command_receiver) = mpsc::channel(1);
        let artifact_service =
            ArtifactService::new(&tmp_dir, build_command_sender.clone(), p2p_client)
                .expect("Creating ArtifactService failed");

        let build_service = BuildService::new(&tmp_dir, build_command_sender, "", "")
            .expect("Creating BuildService failed");

        let filter = make_node_routes(
            Arc::new(Mutex::new(artifact_service)),
            Arc::new(Mutex::new(build_service)),
        );
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

        let (build_command_sender, _build_command_receiver) = mpsc::channel(1);
        let artifact_service =
            ArtifactService::new(&tmp_dir, build_command_sender.clone(), p2p_client)
                .expect("Creating ArtifactService failed");

        let build_service = BuildService::new(&tmp_dir, build_command_sender, "", "")
            .expect("Creating BuildService failed");

        let filter = make_node_routes(
            Arc::new(Mutex::new(artifact_service)),
            Arc::new(Mutex::new(build_service)),
        );
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
