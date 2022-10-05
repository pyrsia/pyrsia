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

use super::handlers::maven_artifacts::handle_get_maven_artifact;
use crate::artifact_service::service::ArtifactService;
use log::debug;
use warp::Filter;

pub fn make_maven_routes(
    artifact_service: ArtifactService,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let artifact_service_filter = warp::any().map(move || artifact_service.clone());

    let maven2_root = warp::path("maven2")
        .and(warp::path::full())
        .map(|path: warp::path::FullPath| {
            let full_path: String = path.as_str().to_string();
            debug!("route full path: {}", full_path);
            full_path
        })
        .and(artifact_service_filter)
        .and_then(handle_get_maven_artifact);

    warp::any().and(maven2_root)
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use crate::artifact_service::model::PackageType;
    use crate::blockchain_service::service::BlockchainService;
    use crate::build_service::event::BuildEventClient;
    use crate::docker::error_util::RegistryError;
    use crate::network::client::command::Command;
    use crate::network::client::Client;
    use crate::transparency_log::log::{TransparencyLogError, TransparencyLogService};
    use crate::util::test_util;
    use libp2p::identity::Keypair;
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

    async fn create_transparency_log_service(
        artifact_path: impl AsRef<Path>,
        local_keypair: Keypair,
        p2p_client: Client,
    ) -> TransparencyLogService {
        let blockchain_service =
            create_blockchain_service(&local_keypair, p2p_client, &artifact_path).await;

        TransparencyLogService::new(artifact_path, Arc::new(Mutex::new(blockchain_service)))
            .expect("Creating TransparencyLogService failed")
    }

    #[tokio::test]
    async fn maven_routes() {
        let tmp_dir = test_util::tests::setup();

        let local_keypair = Keypair::generate_ed25519();
        let (_command_receiver, p2p_client) = create_p2p_client(&local_keypair);
        let transparency_log_service =
            create_transparency_log_service(&tmp_dir, local_keypair, p2p_client.clone()).await;

        let (build_event_sender, _build_event_receiver) = mpsc::channel(1);
        let build_event_client = BuildEventClient::new(build_event_sender);
        let artifact_service = ArtifactService::new(
            &tmp_dir,
            transparency_log_service,
            build_event_client,
            p2p_client,
        )
        .expect("Creating ArtifactService failed");

        let filter = make_maven_routes(artifact_service);
        let response = warp::test::request()
            .path("/maven2/com/company/artifact/1.8/artifact-1.8.pom")
            .reply(&filter)
            .await;

        let artifact_not_found_error = TransparencyLogError::ArtifactNotFound {
            package_type: PackageType::Maven2,
            package_specific_artifact_id: "com.company/artifact/1.8/artifact-1.8.pom".to_owned(),
        };
        let expected_error: RegistryError = artifact_not_found_error.into();
        let expected_body = format!("Unhandled rejection: {:?}", expected_error);

        assert_eq!(response.status(), 500);
        assert_eq!(expected_body, str::from_utf8(response.body()).unwrap());

        test_util::tests::teardown(tmp_dir);
    }
}
