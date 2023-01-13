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
) -> impl Filter<Extract = (impl warp::Reply,), Error = warp::Rejection> + Clone {
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
    use crate::docker::error_util::RegistryError;
    use crate::transparency_log::log::TransparencyLogError;
    use crate::util::test_util;
    use std::str;

    #[tokio::test]
    async fn maven_routes() {
        let tmp_dir = test_util::tests::setup();

        let (artifact_service, ..) = test_util::tests::create_artifact_service(&tmp_dir);

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
