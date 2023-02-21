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

use super::model::MappingInfo;
use crate::artifact_service::model::PackageType;
use crate::build_service::error::BuildError;

#[derive(Clone)]
pub struct MappingService {
    mapping_service_endpoint: String,
}

fn remove_last_character(mut string: String) -> String {
    string.pop();
    string
}

impl MappingService {
    pub fn new(mapping_service_endpoint: &str) -> Self {
        MappingService {
            mapping_service_endpoint: match mapping_service_endpoint.ends_with('/') {
                true => remove_last_character(mapping_service_endpoint.to_owned()),
                false => mapping_service_endpoint.to_owned(),
            },
        }
    }

    pub async fn get_mapping(
        &self,
        package_type: PackageType,
        package_specific_id: &str,
    ) -> Result<MappingInfo, BuildError> {
        match package_type {
            PackageType::Docker => Ok(MappingInfo {
                package_type,
                package_specific_id: package_specific_id.to_owned(),
                source_repository: None,
                build_spec_url: None,
            }),
            PackageType::Maven2 => self.get_maven_mapping(package_specific_id).await,
        }
    }

    async fn get_maven_mapping(
        &self,
        package_specific_id: &str,
    ) -> Result<MappingInfo, BuildError> {
        let package_specific_pieces: Vec<&str> = package_specific_id.split(':').collect();

        let group_id = package_specific_pieces[0].replace('.', "/");
        let artifact_id = package_specific_pieces[1];
        let version = package_specific_pieces[2];

        let remote_mapping_url = format!(
            "{}/Maven2/{}/{}/{}/{}-{}.mapping",
            self.mapping_service_endpoint, group_id, artifact_id, version, artifact_id, version
        );

        let client = reqwest::Client::new();
        let response = client
            .get(remote_mapping_url)
            .send()
            .await
            .map_err(|e| BuildError::MappingServiceEndpointRequestFailure(e.to_string()))?;

        if response.status().is_success() {
            response
                .json::<MappingInfo>()
                .await
                .map_err(|e| BuildError::InvalidMappingResponse(e.to_string()))
        } else if response.status() == hyper::StatusCode::NOT_FOUND {
            Err(BuildError::MappingNotFound {
                package_type: PackageType::Maven2,
                package_specific_id: package_specific_id.to_owned(),
            })
        } else {
            Err(BuildError::MappingServiceEndpointFailure(response.status()))
        }
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use crate::build_service::mapping::model::SourceRepository;
    use httptest::{matchers, responders, Expectation, Server};
    use hyper::StatusCode;

    #[test]
    fn service_endpoint_with_trailing_slash() {
        let mapping_service_endpoint = "https://mapping-service.pyrsia.io/";
        let mapping_service = MappingService::new(mapping_service_endpoint);
        assert_eq!(
            mapping_service.mapping_service_endpoint,
            remove_last_character(mapping_service_endpoint.to_owned())
        );
    }

    #[test]
    fn service_endpoint_without_trailing_slash() {
        let mapping_service_endpoint = "https://mapping-service.pyrsia.io";
        let mapping_service = MappingService::new(mapping_service_endpoint);
        assert_eq!(
            mapping_service.mapping_service_endpoint,
            mapping_service_endpoint
        );
    }

    #[tokio::test]
    async fn docker_mapping_info() {
        let mapping_info = MappingInfo {
            package_type: PackageType::Docker,
            package_specific_id:
                "sha256:44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a".to_owned(),
            source_repository: None,
            build_spec_url: None,
        };

        let mapping_service = MappingService::new("");

        let package_type = PackageType::Docker;
        let package_specific_id =
            "sha256:44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a";

        let result = mapping_service
            .get_mapping(package_type, package_specific_id)
            .await;
        assert!(result.is_ok());

        let mapping_info_result = result.unwrap();
        assert_eq!(mapping_info, mapping_info_result);
    }

    #[tokio::test]
    async fn maven_mapping_info() {
        let mapping_info = MappingInfo {
            package_type: PackageType::Maven2,
            package_specific_id: "org.apache.maven:maven:3.8.6".to_owned(),
            source_repository: Some(SourceRepository::Git {
                url: "https://github.com/apache/maven".to_owned(),
                tag: "maven-3.8.6".to_owned()
            }),
            build_spec_url: Some("https://raw.githubusercontent.com/pyrsia/pyrsia-mappings/main/Maven2/org/apache/maven/maven/3.8.6/maven-3.8.6.buildspec".to_owned()),
        };

        let http_server = Server::run();
        http_server.expect(
            Expectation::matching(matchers::request::method_path(
                "GET",
                "/Maven2/org/apache/maven/maven/3.8.6/maven-3.8.6.mapping",
            ))
            .respond_with(responders::json_encoded(&mapping_info)),
        );

        let mapping_service = MappingService::new(&http_server.url("/").to_string());

        let result = mapping_service
            .get_mapping(mapping_info.package_type, &mapping_info.package_specific_id)
            .await;
        assert!(result.is_ok());

        let mapping_info_result = result.unwrap();
        assert_eq!(mapping_info, mapping_info_result);
    }

    #[tokio::test]
    #[should_panic(expected = "InvalidMappingResponse")]
    async fn maven_mapping_invalid_mapping() {
        let http_server = Server::run();
        http_server.expect(
            Expectation::matching(matchers::request::method_path(
                "GET",
                "/Maven2/commons-codec/commons-codec/1.15/commons-codec-1.15.mapping",
            ))
            .respond_with(responders::json_encoded("{}")),
        );

        let mapping_service = MappingService::new(&http_server.url("/").to_string());

        let package_type = PackageType::Maven2;
        let package_specific_id = "commons-codec:commons-codec:1.15";

        mapping_service
            .get_mapping(package_type, package_specific_id)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn maven_mapping_info_unknown_mapping() {
        let http_server = Server::run();
        http_server.expect(
            Expectation::matching(matchers::request::method_path(
                "GET",
                "/Maven2/commons-codec/commons-codec/1.14/commons-codec-1.14.mapping",
            ))
            .respond_with(responders::status_code(404)),
        );

        let mapping_service = MappingService::new(&http_server.url("/").to_string());

        let package_type = PackageType::Maven2;
        let package_specific_id = "commons-codec:commons-codec:1.14";

        let error = mapping_service
            .get_mapping(package_type, package_specific_id)
            .await
            .unwrap_err();

        match error {
            BuildError::MappingNotFound {
                package_type: expected_package_type,
                package_specific_id: expected_package_specific_id,
            } => {
                assert_eq!(package_type, expected_package_type);
                assert_eq!(package_specific_id, expected_package_specific_id);
            }
            _ => panic!("Invalid BuildError: {}", error),
        }
    }

    #[tokio::test]
    async fn maven_mapping_other_server_error() {
        let http_server = Server::run();
        http_server.expect(
            Expectation::matching(matchers::request::method_path(
                "GET",
                "/Maven2/commons-codec/commons-codec/1.14/commons-codec-1.14.mapping",
            ))
            .respond_with(responders::status_code(400)),
        );

        let mapping_service = MappingService::new(&http_server.url("/").to_string());

        let package_type = PackageType::Maven2;
        let package_specific_id = "commons-codec:commons-codec:1.14";

        let error = mapping_service
            .get_mapping(package_type, package_specific_id)
            .await
            .unwrap_err();

        match error {
            BuildError::MappingServiceEndpointFailure(StatusCode::BAD_REQUEST) => {}
            _ => panic!("Invalid BuildError: {}", error),
        }
    }

    #[tokio::test]
    #[should_panic(expected = "MappingServiceEndpointRequestFailure")]
    async fn maven_mapping_http_error() {
        let mapping_service = MappingService::new("");

        let package_type = PackageType::Maven2;
        let package_specific_id = "commons-codec:commons-codec:1.14";

        mapping_service
            .get_mapping(package_type, package_specific_id)
            .await
            .unwrap();
    }
}
