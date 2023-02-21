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

use crate::build_service::error::BuildError;
use crate::build_service::mapping::model::MappingInfo;
use crate::build_service::model::BuildInfo;

#[derive(Clone)]
pub struct PipelineService {
    http_client: reqwest::Client,
    pipeline_service_endpoint: String,
}

fn remove_last_character(mut string: String) -> String {
    string.pop();
    string
}

impl PipelineService {
    pub fn new(pipeline_service_endpoint: &str) -> Self {
        PipelineService {
            http_client: reqwest::Client::new(),
            pipeline_service_endpoint: match pipeline_service_endpoint.ends_with('/') {
                true => remove_last_character(pipeline_service_endpoint.to_owned()),
                false => pipeline_service_endpoint.to_owned(),
            },
        }
    }

    pub async fn start_build(&self, mapping_info: MappingInfo) -> Result<String, BuildError> {
        let start_build_endpoint = format!("{}/build", self.pipeline_service_endpoint);

        let start_build_response = self
            .http_client
            .put(start_build_endpoint)
            .json(&mapping_info)
            .send()
            .await
            .map_err(|e| BuildError::PipelineServiceEndpointRequestFailure(e.to_string()))?;

        if start_build_response.status().is_success() {
            start_build_response
                .json::<String>()
                .await
                .map_err(|e| BuildError::InvalidPipelineResponse(e.to_string()))
        } else {
            Err(BuildError::PipelineServiceEndpointFailure(
                start_build_response.status(),
            ))
        }
    }

    pub async fn get_build_status(&self, build_id: &str) -> Result<BuildInfo, BuildError> {
        let get_build_status_endpoint =
            format!("{}/build/{}", self.pipeline_service_endpoint, build_id);

        let get_build_status_response = self
            .http_client
            .get(get_build_status_endpoint)
            .send()
            .await
            .map_err(|e| BuildError::PipelineServiceEndpointRequestFailure(e.to_string()))?;

        if get_build_status_response.status().is_success() {
            get_build_status_response
                .json::<BuildInfo>()
                .await
                .map_err(|e| BuildError::InvalidPipelineResponse(e.to_string()))
        } else {
            Err(BuildError::PipelineServiceEndpointFailure(
                get_build_status_response.status(),
            ))
        }
    }

    pub async fn download_artifact(&self, artifact_url: &str) -> Result<bytes::Bytes, BuildError> {
        let download_artifact_endpoint =
            format!("{}{}", self.pipeline_service_endpoint, artifact_url);

        let download_artifact_response = self
            .http_client
            .get(download_artifact_endpoint)
            .send()
            .await
            .map_err(|e| BuildError::PipelineServiceEndpointRequestFailure(e.to_string()))?;

        if download_artifact_response.status().is_success() {
            download_artifact_response
                .bytes()
                .await
                .map_err(|e| BuildError::InvalidPipelineResponse(e.to_string()))
        } else {
            Err(BuildError::PipelineServiceEndpointFailure(
                download_artifact_response.status(),
            ))
        }
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use crate::artifact_service::model::PackageType;
    use crate::build_service::model::BuildStatus;
    use httptest::{matchers, responders, Expectation, Server};
    use hyper::StatusCode;

    #[test]
    fn service_endpoint_with_trailing_slash() {
        let pipeline_service_endpoint = "https://pipeline-service.pyrsia.io/";
        let pipeline_service = PipelineService::new(pipeline_service_endpoint);
        assert_eq!(
            pipeline_service.pipeline_service_endpoint,
            remove_last_character(pipeline_service_endpoint.to_owned())
        );
    }

    #[test]
    fn service_endpoint_without_trailing_slash() {
        let pipeline_service_endpoint = "https://pipeline-service.pyrsia.io";
        let pipeline_service = PipelineService::new(pipeline_service_endpoint);
        assert_eq!(
            pipeline_service.pipeline_service_endpoint,
            pipeline_service_endpoint
        );
    }

    #[tokio::test]
    async fn start_build() {
        let mapping_info = MappingInfo {
            package_type: PackageType::Docker,
            package_specific_id:
                "sha256:44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a".to_owned(),
            source_repository: None,
            build_spec_url: None,
        };

        let build_id = uuid::Uuid::new_v4().to_string();

        let http_server = Server::run();
        http_server.expect(
            Expectation::matching(matchers::all_of!(
                matchers::request::method_path("PUT", "/build"),
                matchers::request::body(matchers::json_decoded(matchers::eq(serde_json::json!(
                    &mapping_info
                ))))
            ))
            .respond_with(responders::json_encoded(&build_id)),
        );

        let pipeline_service = PipelineService::new(&http_server.url("/").to_string());

        let build_id_result = pipeline_service.start_build(mapping_info).await.unwrap();
        assert_eq!(build_id_result, build_id);
    }

    #[tokio::test]
    #[should_panic(expected = "InvalidPipelineResponse")]
    async fn start_build_invalid_response() {
        let mapping_info = MappingInfo {
            package_type: PackageType::Docker,
            package_specific_id:
                "sha256:44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a".to_owned(),
            source_repository: None,
            build_spec_url: None,
        };

        let http_server = Server::run();
        http_server.expect(
            Expectation::matching(matchers::all_of!(
                matchers::request::method_path("PUT", "/build"),
                matchers::request::body(matchers::json_decoded(matchers::eq(serde_json::json!(
                    &mapping_info
                ))))
            ))
            .respond_with(responders::status_code(200).body(bytes::Bytes::from("{}"))),
        );

        let pipeline_service = PipelineService::new(&http_server.url("/").to_string());

        pipeline_service.start_build(mapping_info).await.unwrap();
    }

    #[tokio::test]
    async fn start_build_server_error() {
        let mapping_info = MappingInfo {
            package_type: PackageType::Docker,
            package_specific_id:
                "sha256:44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a".to_owned(),
            source_repository: None,
            build_spec_url: None,
        };

        let http_server = Server::run();
        http_server.expect(
            Expectation::matching(matchers::all_of!(
                matchers::request::method_path("PUT", "/build"),
                matchers::request::body(matchers::json_decoded(matchers::eq(serde_json::json!(
                    &mapping_info
                ))))
            ))
            .respond_with(responders::status_code(400)),
        );

        let pipeline_service = PipelineService::new(&http_server.url("/").to_string());

        let error = pipeline_service
            .start_build(mapping_info)
            .await
            .unwrap_err();

        match error {
            BuildError::PipelineServiceEndpointFailure(StatusCode::BAD_REQUEST) => {}
            _ => panic!("Invalid BuildError: {}", error),
        }
    }

    #[tokio::test]
    #[should_panic(expected = "PipelineServiceEndpointRequestFailure")]
    async fn start_build_http_error() {
        let mapping_info = MappingInfo {
            package_type: PackageType::Docker,
            package_specific_id:
                "sha256:44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a".to_owned(),
            source_repository: None,
            build_spec_url: None,
        };

        let pipeline_service = PipelineService::new("");

        pipeline_service.start_build(mapping_info).await.unwrap();
    }

    #[tokio::test]
    async fn get_build_status() {
        let build_id = uuid::Uuid::new_v4().to_string();

        let build_info = BuildInfo {
            id: build_id.clone(),
            status: BuildStatus::Running,
        };

        let http_server = Server::run();
        http_server.expect(
            Expectation::matching(matchers::request::method_path(
                "GET",
                format!("/build/{}", &build_id),
            ))
            .respond_with(responders::json_encoded(&build_info)),
        );

        let pipeline_service = PipelineService::new(&http_server.url("/").to_string());

        let build_info_result = pipeline_service.get_build_status(&build_id).await.unwrap();
        assert_eq!(build_info_result, build_info);
    }

    #[tokio::test]
    #[should_panic(expected = "InvalidPipelineResponse")]
    async fn get_build_status_invalid_response() {
        let build_id = uuid::Uuid::new_v4().to_string();

        let http_server = Server::run();
        http_server.expect(
            Expectation::matching(matchers::request::method_path(
                "GET",
                format!("/build/{}", &build_id),
            ))
            .respond_with(responders::json_encoded("{}")),
        );

        let pipeline_service = PipelineService::new(&http_server.url("/").to_string());

        pipeline_service.get_build_status(&build_id).await.unwrap();
    }

    #[tokio::test]
    async fn get_build_status_server_error() {
        let build_id = uuid::Uuid::new_v4().to_string();

        let http_server = Server::run();
        http_server.expect(
            Expectation::matching(matchers::request::method_path(
                "GET",
                format!("/build/{}", &build_id),
            ))
            .respond_with(responders::status_code(400)),
        );

        let pipeline_service = PipelineService::new(&http_server.url("/").to_string());

        let error = pipeline_service
            .get_build_status(&build_id)
            .await
            .unwrap_err();

        match error {
            BuildError::PipelineServiceEndpointFailure(StatusCode::BAD_REQUEST) => {}
            _ => panic!("Invalid BuildError: {}", error),
        }
    }

    #[tokio::test]
    #[should_panic(expected = "PipelineServiceEndpointRequestFailure")]
    async fn get_build_status_http_error() {
        let pipeline_service = PipelineService::new("");

        pipeline_service
            .get_build_status(&uuid::Uuid::new_v4().to_string())
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn download_artifact() {
        let artifact_url = "/artifact.file";
        let artifact_bytes = bytes::Bytes::from("some_bytes");

        let http_server = Server::run();
        http_server.expect(
            Expectation::matching(matchers::request::method_path("GET", artifact_url))
                .respond_with(responders::status_code(200).body(artifact_bytes.clone())),
        );

        let pipeline_service = PipelineService::new(&http_server.url("/").to_string());

        let download_artifact_result = pipeline_service
            .download_artifact(artifact_url)
            .await
            .unwrap();
        assert_eq!(download_artifact_result, artifact_bytes);
    }

    #[tokio::test]
    async fn download_artifact_server_error() {
        let artifact_url = "/artifact.file";

        let http_server = Server::run();
        http_server.expect(
            Expectation::matching(matchers::request::method_path("GET", artifact_url))
                .respond_with(responders::status_code(400)),
        );

        let pipeline_service = PipelineService::new(&http_server.url("/").to_string());

        let error = pipeline_service
            .download_artifact(artifact_url)
            .await
            .unwrap_err();

        match error {
            BuildError::PipelineServiceEndpointFailure(StatusCode::BAD_REQUEST) => {}
            _ => panic!("Invalid BuildError: {}", error),
        }
    }

    #[tokio::test]
    #[should_panic(expected = "PipelineServiceEndpointRequestFailure")]
    async fn download_artifact_http_error() {
        let pipeline_service = PipelineService::new("");

        pipeline_service
            .download_artifact("/artifact.file")
            .await
            .unwrap();
    }
}
