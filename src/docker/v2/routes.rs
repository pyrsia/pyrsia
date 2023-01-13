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
use warp::Filter;

pub fn make_docker_routes(
    artifact_service: ArtifactService,
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
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

    let v2_manifests_get = warp::path!("v2" / "library" / String / "manifests" / String)
        .and(warp::get())
        .and(artifact_service_filter.clone())
        .and_then(fetch_manifest_or_build);

    let v2_manifests_head = warp::path!("v2" / "library" / String / "manifests" / String)
        .and(warp::head())
        .and(artifact_service_filter.clone())
        .and_then(fetch_manifest);

    let v2_blobs = warp::path!("v2" / "library" / String / "blobs" / String)
        .and(warp::get())
        .and(warp::path::end())
        .and(artifact_service_filter)
        .and_then(handle_get_blobs);

    warp::any().and(
        v2_base
            .or(v2_manifests_get)
            .or(v2_manifests_head)
            .or(v2_blobs),
    )
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use crate::docker::error_util::{RegistryError, RegistryErrorCode};
    use crate::util::test_util;
    use std::str;

    #[tokio::test]
    async fn docker_routes_base() {
        let tmp_dir = test_util::tests::setup();

        let (artifact_service, ..) = test_util::tests::create_artifact_service(&tmp_dir);

        let filter = make_docker_routes(artifact_service);
        let response = warp::test::request().path("/v2").reply(&filter).await;

        let expected_body = "{}";

        assert_eq!(response.status(), 200);
        assert_eq!(expected_body, str::from_utf8(response.body()).unwrap());

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn docker_routes_blobs() {
        let tmp_dir = test_util::tests::setup();

        let (artifact_service, ..) = test_util::tests::create_artifact_service(&tmp_dir);

        let filter = make_docker_routes(artifact_service);
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

        let (artifact_service, ..) = test_util::tests::create_artifact_service(&tmp_dir);

        let filter = make_docker_routes(artifact_service);
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
