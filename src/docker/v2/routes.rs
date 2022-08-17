// all warp routes can be here
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

use crate::artifact_service::service::ArtifactService;

use super::handlers::blobs::*;
use super::handlers::manifests::*;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::Filter;

pub fn make_docker_routes(
    artifact_service: Arc<Mutex<ArtifactService>>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let empty_json = "{}";
    let v2_base = warp::path("v2")
        .and(warp::get())
        .and(warp::path::end())
        .map(move || empty_json)
        .with(warp::reply::with::header(
            "Content-Length",
            empty_json.len(),
        ))
        .with(warp::reply::with::header(
            "Content-Type",
            "application/json",
        ));

    let artifact_service_filter = warp::any().map(move || artifact_service.clone());

    let v2_manifests = warp::path!("v2" / "library" / String / "manifests" / String)
        .and(warp::get().or(warp::head()).unify())
        .and(artifact_service_filter.clone())
        .and_then(fetch_manifest);

    let v2_blobs = warp::path!("v2" / "library" / String / "blobs" / String)
        .and(warp::get().or(warp::head()).unify())
        .and(warp::path::end())
        .and(artifact_service_filter)
        .and_then(handle_get_blobs);

    warp::any().and(v2_base.or(v2_manifests).or(v2_blobs))
}

#[cfg(all(test, not(tarpaulin_include)))]
mod tests {
    use super::*;
    use crate::build_service::event::BuildEventClient;
    use crate::docker::error_util::{RegistryError, RegistryErrorCode};
    use crate::network::client::Client;
    use crate::util::test_util;
    use libp2p::identity::Keypair;
    use std::str;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn docker_routes_base() {
        let tmp_dir = test_util::tests::setup();

        let (command_sender, _command_receiver) = mpsc::channel(1);
        let p2p_client = Client {
            sender: command_sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
        };

        let (build_event_sender, _build_event_receiver) = mpsc::channel(1);
        let build_event_client = BuildEventClient::new(build_event_sender);
        let artifact_service = ArtifactService::new(&tmp_dir, build_event_client, p2p_client)
            .expect("Creating ArtifactService failed");

        let filter = make_docker_routes(Arc::new(Mutex::new(artifact_service)));
        let response = warp::test::request().path("/v2").reply(&filter).await;

        let expected_body = "{}";

        assert_eq!(response.status(), 200);
        assert_eq!(expected_body, str::from_utf8(response.body()).unwrap());

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn docker_routes_blobs() {
        let tmp_dir = test_util::tests::setup();

        let (command_sender, _command_receiver) = mpsc::channel(1);
        let p2p_client = Client {
            sender: command_sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
        };

        let (build_event_sender, _build_event_receiver) = mpsc::channel(1);
        let build_event_client = BuildEventClient::new(build_event_sender);
        let artifact_service = ArtifactService::new(&tmp_dir, build_event_client, p2p_client)
            .expect("Creating ArtifactService failed");

        let filter = make_docker_routes(Arc::new(Mutex::new(artifact_service)));
        let response = warp::test::request()
            .path("/v2/library/alpine/blobs/sha256:44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a")
            .reply(&filter)
            .await;

        let expected_error = RegistryError {
            code: RegistryErrorCode::BlobUnknown,
        };
        let expected_body = format!("Unhandled rejection: {:?}", expected_error);

        assert_eq!(response.status(), 500);
        assert_eq!(expected_body, str::from_utf8(response.body()).unwrap());

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn docker_routes_manifests() {
        let tmp_dir = test_util::tests::setup();

        let (command_sender, _command_receiver) = mpsc::channel(1);
        let p2p_client = Client {
            sender: command_sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
        };

        let (build_event_sender, _build_event_receiver) = mpsc::channel(1);
        let build_event_client = BuildEventClient::new(build_event_sender);
        let artifact_service = ArtifactService::new(&tmp_dir, build_event_client, p2p_client)
            .expect("Creating ArtifactService failed");

        let filter = make_docker_routes(Arc::new(Mutex::new(artifact_service)));
        let response = warp::test::request()
            .path("/v2/library/alpine/manifests/1.15")
            .reply(&filter)
            .await;

        let expected_error = RegistryError {
            code: RegistryErrorCode::ManifestUnknown,
        };
        let expected_body = format!("Unhandled rejection: {:?}", expected_error);

        assert_eq!(response.status(), 500);
        assert_eq!(expected_body, str::from_utf8(response.body()).unwrap());

        test_util::tests::teardown(tmp_dir);
    }
}
