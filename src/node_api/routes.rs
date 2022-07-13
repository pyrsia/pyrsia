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
    use crate::build_service::model::{BuildInfo, BuildStatus};
    use crate::docker::error_util::{RegistryError, RegistryErrorCode};
    use crate::network::client::Client;
    use crate::util::test_util;
    use libp2p::identity::Keypair;
    use std::str;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn node_routes_build_docker() {
        let tmp_dir = test_util::tests::setup();

        let (sender, _) = mpsc::channel(1);
        let p2p_client = Client {
            sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
        };

        let artifact_service =
            ArtifactService::new(&tmp_dir, p2p_client).expect("Creating ArtifactService failed");

        let filter = make_node_routes(Arc::new(Mutex::new(artifact_service)));
        let request = RequestDockerBuild {
            manifest: "alpine/1.15".to_owned(),
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

        let (sender, _) = mpsc::channel(1);
        let p2p_client = Client {
            sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
        };

        let artifact_service =
            ArtifactService::new(&tmp_dir, p2p_client).expect("Creating ArtifactService failed");

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

        let build_info: BuildInfo = serde_json::from_slice(response.body()).unwrap();
        assert_eq!(build_info.status, BuildStatus::Running);

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn node_routes_peers() {
        let tmp_dir = test_util::tests::setup();

        let (sender, _) = mpsc::channel(1);
        let p2p_client = Client {
            sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
        };

        let artifact_service =
            ArtifactService::new(&tmp_dir, p2p_client).expect("Creating ArtifactService failed");

        let filter = make_node_routes(Arc::new(Mutex::new(artifact_service)));
        let response = warp::test::request().path("/peers").reply(&filter).await;

        let expected_error = RegistryError {
            code: RegistryErrorCode::Unknown("channel closed".to_owned()),
        };
        let expected_body = format!("Unhandled rejection: {:?}", expected_error);

        assert_eq!(response.status(), 500);
        assert_eq!(expected_body, str::from_utf8(response.body()).unwrap());

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn node_routes_status() {
        let tmp_dir = test_util::tests::setup();

        let (sender, _) = mpsc::channel(1);
        let p2p_client = Client {
            sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
        };

        let artifact_service =
            ArtifactService::new(&tmp_dir, p2p_client).expect("Creating ArtifactService failed");

        let filter = make_node_routes(Arc::new(Mutex::new(artifact_service)));
        let response = warp::test::request().path("/status").reply(&filter).await;

        let expected_error = RegistryError {
            code: RegistryErrorCode::Unknown("channel closed".to_owned()),
        };
        let expected_body = format!("Unhandled rejection: {:?}", expected_error);

        assert_eq!(response.status(), 500);
        assert_eq!(expected_body, str::from_utf8(response.body()).unwrap());

        test_util::tests::teardown(tmp_dir);
    }
}
