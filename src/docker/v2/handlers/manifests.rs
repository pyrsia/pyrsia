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

use crate::artifact_manager;
use crate::artifact_manager::HashAlgorithm;
use crate::docker::docker_hub_util::get_docker_hub_auth_token;
use crate::docker::error_util::{RegistryError, RegistryErrorCode};
use crate::node_manager::model::artifact::{Artifact, ArtifactBuilder};
use crate::node_manager::model::package_type::PackageTypeName;
use crate::node_manager::model::package_version::{PackageVersion, PackageVersionBuilder};
use anyhow::{anyhow, Context};
use bytes::Bytes;
use easy_hasher::easy_hasher::{file_hash, raw_sha256, raw_sha512, Hash};
use log::{debug, error, info, warn};
use reqwest::{header, Client};
use serde_json::{Map, Value};
use std::fs;
use uuid::Uuid;
use warp::http::StatusCode;
use warp::{Rejection, Reply};

// Handles GET endpoint documented at https://docs.docker.com/registry/spec/api/#manifest
pub async fn handle_get_manifests(name: String, tag: String) -> Result<impl Reply, Rejection> {
    let mut hash = String::from(&tag);
    if let None = tag.find(':') {
        let manifest = format!(
            "/tmp/registry/docker/registry/v2/repositories/{}/_manifests/tags/{}/current/link",
            name, tag
        );

        match fs::read_to_string(manifest) {
            Ok(local_hash) => hash = local_hash,
            Err(_) => hash = get_manifest_from_docker_hub(&name, &tag).await?,
        }
    }

    let blob = format!(
        "/tmp/registry/docker/registry/v2/blobs/sha256/{}/{}/data",
        hash.get(7..9).unwrap(),
        hash.get(7..).unwrap()
    );
    let blob_content = fs::read_to_string(blob);
    if blob_content.is_err() {
        return Err(warp::reject::custom(RegistryError {
            code: RegistryErrorCode::ManifestUnknown,
        }));
    }

    let content = blob_content.unwrap();
    Ok(warp::http::response::Builder::new()
        .header(
            "Content-Type",
            "application/vnd.docker.distribution.manifest.v2+json",
        )
        .header("Content-Length", content.len())
        .status(StatusCode::OK)
        .body(content)
        .unwrap())
}

const LOCATION: &str = "Location";

// Handles PUT endpoint documented at https://docs.docker.com/registry/spec/api/#manifest
pub async fn handle_put_manifest(
    name: String,
    reference: String,
    bytes: Bytes,
) -> Result<impl Reply, Rejection> {
    match store_manifest_in_artifact_manager(&bytes) {
        Ok(artifact_hash) => {
            info!(
                "Stored manifest with {} hash {}",
                artifact_hash.0,
                hex::encode(artifact_hash.1)
            );
            let package_version =
                match package_version_from_manifest_bytes(&bytes, &name, &reference) {
                    Ok(pv) => pv,
                    Err(error) => {
                        let err_string = error.to_string();
                        error!("{}", err_string);
                        return Err(warp::reject::custom(RegistryError {
                            code: RegistryErrorCode::Unknown(err_string),
                        }));
                    }
                };
            info!(
                "Created PackageVersion from manifest: {:?}",
                package_version
            );
        }
        Err(error) => warn!("Error storing manifest in artifact_manager {}", error),
    };

    let hash = store_manifest_in_filesystem(&name, &reference, bytes)?;

    Ok(warp::http::response::Builder::new()
        .header(
            LOCATION,
            format!(
                "http://localhost:7878/v2/{}/manifests/sha256:{}",
                name, hash
            ),
        )
        .header("Docker-Content-Digest", format!("sha256:{}", hash))
        .status(StatusCode::CREATED)
        .body("")
        .unwrap())
}

fn store_manifest_in_filesystem(
    name: &str,
    reference: &str,
    bytes: Bytes,
) -> Result<String, Rejection> {
    let id = Uuid::new_v4();

    // temporary upload of manifest
    let blob_upload_dest_dir = format!(
        "/tmp/registry/docker/registry/v2/repositories/{}/_uploads/{}",
        name, id
    );
    fs::create_dir_all(&blob_upload_dest_dir).map_err(RegistryError::from)?;

    let blob_upload_dest = format!(
        "/tmp/registry/docker/registry/v2/repositories/{}/_uploads/{}/data",
        name, id
    );

    super::blobs::append_to_blob(&blob_upload_dest, bytes).map_err(RegistryError::from)?;

    // calculate sha256 checksum on manifest file
    let file256 = file_hash(raw_sha256, &blob_upload_dest);
    let digest: Hash;
    match file256 {
        Ok(hash) => digest = hash,
        Err(e) => {
            return Err(warp::reject::custom(RegistryError {
                code: RegistryErrorCode::Unknown(e),
            }))
        }
    }

    let hash = digest.to_hex_string();
    debug!(
        "Generated hash for manifest {}/{}: {}",
        name, reference, hash
    );
    let mut blob_dest = format!(
        "/tmp/registry/docker/registry/v2/blobs/sha256/{}/{}",
        hash.get(0..2).unwrap(),
        hash
    );
    fs::create_dir_all(&blob_dest).map_err(RegistryError::from)?;
    blob_dest.push_str("/data");

    // copy temporary upload to final blob location
    fs::copy(&blob_upload_dest, &blob_dest).map_err(RegistryError::from)?;

    // remove temporary files
    fs::remove_dir_all(blob_upload_dest_dir).map_err(RegistryError::from)?;

    // create manifest link file in revisions
    let mut manifest_rev_dest = format!(
        "/tmp/registry/docker/registry/v2/repositories/{}/_manifests/revisions/sha256/{}",
        name, hash
    );
    fs::create_dir_all(&manifest_rev_dest).map_err(RegistryError::from)?;
    manifest_rev_dest.push_str("/link");
    fs::write(manifest_rev_dest, format!("sha256:{}", hash)).map_err(RegistryError::from)?;

    // create manifest link file in tags if reference is a tag (no colon)
    if let None = reference.find(':') {
        let mut manifest_tag_dest = format!(
            "/tmp/registry/docker/registry/v2/repositories/{}/_manifests/tags/{}/current",
            name, reference
        );
        fs::create_dir_all(&manifest_tag_dest).map_err(RegistryError::from)?;
        manifest_tag_dest.push_str("/link");
        fs::write(manifest_tag_dest, format!("sha256:{}", hash)).map_err(RegistryError::from)?;
    }

    Ok(hash)
}

async fn get_manifest_from_docker_hub(name: &str, tag: &str) -> Result<String, Rejection> {
    let token = get_docker_hub_auth_token(name).await?;

    get_manifest_from_docker_hub_with_token(name, tag, token).await
}

async fn get_manifest_from_docker_hub_with_token(
    name: &str,
    tag: &str,
    token: String,
) -> Result<String, Rejection> {
    let url = format!(
        "https://registry-1.docker.io/v2/library/{}/manifests/{}",
        name, tag
    );

    debug!("Reading manifest from docker.io with url: {}", url);
    let response = Client::new()
        .get(url)
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(
            "Accept",
            "application/vnd.docker.distribution.manifest.v2+json",
        )
        .send()
        .await
        .map_err(RegistryError::from)?;

    debug!(
        "Got manifest from docker.io with status {}",
        response.status()
    );

    let bytes = response.bytes().await.map_err(RegistryError::from)?;

    let hash = store_manifest_in_filesystem(name, tag, bytes)?;

    Ok(hash)
}

fn store_manifest_in_artifact_manager(bytes: &Bytes) -> anyhow::Result<(HashAlgorithm, Vec<u8>)> {
    let manifest_vec = bytes.to_vec();
    let sha512: Vec<u8> = raw_sha512(manifest_vec.clone()).to_vec();
    let artifact_hash = artifact_manager::Hash::new(HashAlgorithm::SHA512, &sha512)?;
    crate::node_manager::handlers::ART_MGR
        .push_artifact(&mut manifest_vec.as_slice(), &artifact_hash)?;
    Ok((HashAlgorithm::SHA512, sha512))
}

const DOCKER_NAMESPACE_ID: &str = "4658011310974e1bb5c46fd4df7e78b9";

fn package_version_from_manifest_bytes(
    bytes: &Bytes,
    docker_name: &str,
    docker_reference: &str,
) -> Result<PackageVersion, anyhow::Error> {
    let json_string = String::from_utf8(bytes.to_vec())?;
    match serde_json::from_str::<Value>(&json_string) {
        Ok(Value::Object(json_object)) => package_version_from_manifest_json(
            &json_object,
            &json_string,
            docker_name,
            docker_reference,
        ),
        Ok(_) => invalid_manifest(&json_string),
        Err(error) => Err(anyhow!(
            "Error parsing docker manifest JSON: {} in {}",
            error,
            json_string
        )),
    }
}

fn package_version_from_manifest_json(
    json_object: &Map<String, Value>,
    json_string: &str,
    docker_name: &str,
    docker_reference: &str,
) -> Result<PackageVersion, anyhow::Error> {
    let result = match manifest_schema_version(json_object, json_string)? {
        1 => package_version_from_schema1(json_object),
        2 => package_version_from_schema2(json_object, json_string, docker_name, docker_reference),
        n => Err(anyhow!("Unsupported manifest schema version {}", n)),
    };
    if result.is_err() {
        error!("Invalid manifest {}", json_string)
    }
    result
}

const FS_LAYERS: &str = "fsLayers";

const MIME_TYPE_BLOB_GZIPPED: &str = "application/vnd.docker.image.rootfs.diff.tar.gzip";

fn package_version_from_schema1(
    json_object: &Map<String, Value>,
) -> Result<PackageVersion, anyhow::Error> {
    let manifest_name = json_object
        .get("name")
        .context("missing name field")?
        .as_str()
        .context("invalid name")?;
    let manifest_tag = json_object
        .get("tag")
        .context("missing tag field")?
        .as_str()
        .context("invalid tag")?;
    let fslayers = json_object
        .get(FS_LAYERS)
        .context("missing fsLayers field")?
        .as_array()
        .context("invalid fsLayers")?;
    let mut artifacts: Vec<Artifact> = Vec::new();
    for fslayer in fslayers {
        add_fslayers(&mut artifacts, fslayer)?;
    }
    Ok(PackageVersionBuilder::default()
        .id(String::from(
            Uuid::new_v4()
                .to_simple()
                .encode_lower(&mut Uuid::encode_buffer()),
        ))
        .namespace_id(DOCKER_NAMESPACE_ID.to_string())
        .name(String::from(manifest_name))
        .pkg_type(PackageTypeName::Docker)
        .version(String::from(manifest_tag))
        .artifacts(artifacts)
        .build()?)
}

fn add_fslayers(artifacts: &mut Vec<Artifact>, fslayer: &Value) -> Result<(), anyhow::Error> {
    let hex_digest = fslayer
        .as_object()
        .context("invalid fslayer")?
        .get("blobSum")
        .context("missing blobSum")?
        .as_str()
        .context("invalid blobSum")?;
    if !hex_digest.starts_with("sha256:") {
        return Err(anyhow!("Only sha256 digests are supported: {}", hex_digest));
    }
    let digest = hex::decode(&hex_digest["sha256:".len()..])?;
    artifacts.push(
        ArtifactBuilder::default()
            .algorithm(HashAlgorithm::SHA256)
            .hash(digest)
            .mime_type(MIME_TYPE_BLOB_GZIPPED.to_string())
            .build()?,
    );
    Ok(())
}

fn package_version_from_schema2(
    _json_object: &Map<String, Value>,
    _json_string: &str,
    _docker_name: &str,
    _docker_reference: &str,
) -> Result<PackageVersion, anyhow::Error> {
    todo!()
}

fn manifest_schema_version(
    json_object: &Map<String, Value>,
    json_string: &str,
) -> Result<u64, anyhow::Error> {
    match json_object.get("schemaVersion") {
        Some(Value::Number(n)) => match n.as_u64() {
            Some(version) => Ok(version),
            None => invalid_manifest(json_string),
        },
        Some(Value::String(s)) => s
            .as_str()
            .parse::<u64>()
            .with_context(|| format!("Invalid schemaVersion value: {}", s)),
        _ => invalid_manifest(json_string),
    }
}

fn invalid_manifest<T>(json_string: &str) -> Result<T, anyhow::Error> {
    Err(anyhow!("Invalid JSON manifest: {}", json_string))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_manager::handlers::ART_MGR;
    use bytes::Bytes;
    use futures::executor;
    use serde::de::StdError;

    const MANIFEST_V1_JSON: &str = r##"{
   "name": "hello-world",
   "tag": "latest",
   "architecture": "amd64",
   "fsLayers": [
      {
         "blobSum": "sha256:5f70bf18a086007016e948b04aed3b82103a36bea41755b6cddfaf10ace3c6ef"
      },
      {
         "blobSum": "sha256:5f70bf18a086007016e948b04aed3b82103a36bea41755b6cddfaf10ace3c6ef"
      },
      {
         "blobSum": "sha256:cc8567d70002e957612902a8e985ea129d831ebe04057d88fb644857caa45d11"
      },
      {
         "blobSum": "sha256:5f70bf18a086007016e948b04aed3b82103a36bea41755b6cddfaf10ace3c6ef"
      }
   ],
   "history": [
      {
         "v1Compatibility": "{\"id\":\"e45a5af57b00862e5ef5782a9925979a02ba2b12dff832fd0991335f4a11e5c5\",\"parent\":\"31cbccb51277105ba3ae35ce33c22b69c9e3f1002e76e4c736a2e8ebff9d7b5d\",\"created\":\"2014-12-31T22:57:59.178729048Z\",\"container\":\"27b45f8fb11795b52e9605b686159729b0d9ca92f76d40fb4f05a62e19c46b4f\",\"container_config\":{\"Hostname\":\"8ce6509d66e2\",\"Domainname\":\"\",\"User\":\"\",\"Memory\":0,\"MemorySwap\":0,\"CpuShares\":0,\"Cpuset\":\"\",\"AttachStdin\":false,\"AttachStdout\":false,\"AttachStderr\":false,\"PortSpecs\":null,\"ExposedPorts\":null,\"Tty\":false,\"OpenStdin\":false,\"StdinOnce\":false,\"Env\":[\"PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin\"],\"Cmd\":[\"/bin/sh\",\"-c\",\"#(nop) CMD [/hello]\"],\"Image\":\"31cbccb51277105ba3ae35ce33c22b69c9e3f1002e76e4c736a2e8ebff9d7b5d\",\"Volumes\":null,\"WorkingDir\":\"\",\"Entrypoint\":null,\"NetworkDisabled\":false,\"MacAddress\":\"\",\"OnBuild\":[],\"SecurityOpt\":null,\"Labels\":null},\"docker_version\":\"1.4.1\",\"config\":{\"Hostname\":\"8ce6509d66e2\",\"Domainname\":\"\",\"User\":\"\",\"Memory\":0,\"MemorySwap\":0,\"CpuShares\":0,\"Cpuset\":\"\",\"AttachStdin\":false,\"AttachStdout\":false,\"AttachStderr\":false,\"PortSpecs\":null,\"ExposedPorts\":null,\"Tty\":false,\"OpenStdin\":false,\"StdinOnce\":false,\"Env\":[\"PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin\"],\"Cmd\":[\"/hello\"],\"Image\":\"31cbccb51277105ba3ae35ce33c22b69c9e3f1002e76e4c736a2e8ebff9d7b5d\",\"Volumes\":null,\"WorkingDir\":\"\",\"Entrypoint\":null,\"NetworkDisabled\":false,\"MacAddress\":\"\",\"OnBuild\":[],\"SecurityOpt\":null,\"Labels\":null},\"architecture\":\"amd64\",\"os\":\"linux\",\"Size\":0}\n"
      },
      {
         "v1Compatibility": "{\"id\":\"e45a5af57b00862e5ef5782a9925979a02ba2b12dff832fd0991335f4a11e5c5\",\"parent\":\"31cbccb51277105ba3ae35ce33c22b69c9e3f1002e76e4c736a2e8ebff9d7b5d\",\"created\":\"2014-12-31T22:57:59.178729048Z\",\"container\":\"27b45f8fb11795b52e9605b686159729b0d9ca92f76d40fb4f05a62e19c46b4f\",\"container_config\":{\"Hostname\":\"8ce6509d66e2\",\"Domainname\":\"\",\"User\":\"\",\"Memory\":0,\"MemorySwap\":0,\"CpuShares\":0,\"Cpuset\":\"\",\"AttachStdin\":false,\"AttachStdout\":false,\"AttachStderr\":false,\"PortSpecs\":null,\"ExposedPorts\":null,\"Tty\":false,\"OpenStdin\":false,\"StdinOnce\":false,\"Env\":[\"PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin\"],\"Cmd\":[\"/bin/sh\",\"-c\",\"#(nop) CMD [/hello]\"],\"Image\":\"31cbccb51277105ba3ae35ce33c22b69c9e3f1002e76e4c736a2e8ebff9d7b5d\",\"Volumes\":null,\"WorkingDir\":\"\",\"Entrypoint\":null,\"NetworkDisabled\":false,\"MacAddress\":\"\",\"OnBuild\":[],\"SecurityOpt\":null,\"Labels\":null},\"docker_version\":\"1.4.1\",\"config\":{\"Hostname\":\"8ce6509d66e2\",\"Domainname\":\"\",\"User\":\"\",\"Memory\":0,\"MemorySwap\":0,\"CpuShares\":0,\"Cpuset\":\"\",\"AttachStdin\":false,\"AttachStdout\":false,\"AttachStderr\":false,\"PortSpecs\":null,\"ExposedPorts\":null,\"Tty\":false,\"OpenStdin\":false,\"StdinOnce\":false,\"Env\":[\"PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin\"],\"Cmd\":[\"/hello\"],\"Image\":\"31cbccb51277105ba3ae35ce33c22b69c9e3f1002e76e4c736a2e8ebff9d7b5d\",\"Volumes\":null,\"WorkingDir\":\"\",\"Entrypoint\":null,\"NetworkDisabled\":false,\"MacAddress\":\"\",\"OnBuild\":[],\"SecurityOpt\":null,\"Labels\":null},\"architecture\":\"amd64\",\"os\":\"linux\",\"Size\":0}\n"
      }
   ],
   "schemaVersion": 1,
   "signatures": [
      {
         "header": {
            "jwk": {
               "crv": "P-256",
               "kid": "OD6I:6DRK:JXEJ:KBM4:255X:NSAA:MUSF:E4VM:ZI6W:CUN2:L4Z6:LSF4",
               "kty": "EC",
               "x": "3gAwX48IQ5oaYQAYSxor6rYYc_6yjuLCjtQ9LUakg4A",
               "y": "t72ge6kIA1XOjqjVoEOiPPAURltJFBMGDSQvEGVB010"
            },
            "alg": "ES256"
         },
         "signature": "XREm0L8WNn27Ga_iE_vRnTxVMhhYY0Zst_FfkKopg6gWSoTOZTuW4rK0fg_IqnKkEKlbD83tD46LKEGi5aIVFg",
         "protected": "eyJmb3JtYXRMZW5ndGgiOjY2MjgsImZvcm1hdFRhaWwiOiJDbjAiLCJ0aW1lIjoiMjAxNS0wNC0wOFQxODo1Mjo1OVoifQ"
      }]}"##;

    #[test]
    fn happy_put_manifest() -> Result<(), Box<dyn StdError>> {
        let name = "httpbin";
        let reference = "latest";

        let future = async {
            handle_put_manifest(
                name.to_string(),
                reference.to_string(),
                Bytes::from(MANIFEST_V1_JSON.as_bytes()),
            )
            .await
        };
        let result = executor::block_on(future);
        check_put_manifest_result(result);
        check_artifact_manager_side_effects()?;
        Ok(())
    }

    fn check_put_manifest_result(result: Result<impl Reply, Rejection>) {
        match result {
            Ok(reply) => {
                let response = reply.into_response();
                assert_eq!(response.status(), 201);
                assert!(response.headers().contains_key(LOCATION));
            }
            Err(_) => {
                assert!(false)
            }
        };
    }

    fn check_artifact_manager_side_effects() -> Result<(), Box<dyn StdError>> {
        let manifest_sha512: Vec<u8> = raw_sha512(MANIFEST_V1_JSON.as_bytes().to_vec()).to_vec();
        let artifact_hash = artifact_manager::Hash::new(HashAlgorithm::SHA512, &manifest_sha512)?;
        ART_MGR.pull_artifact(&artifact_hash)?;
        Ok(())
    }

    #[test]
    fn package_version_from_manifest() -> Result<(), anyhow::Error> {
        let json_bytes = Bytes::from(MANIFEST_V1_JSON);
        let package_version = package_version_from_manifest_bytes(&json_bytes, "test_pkg", "v1.4")?;
        assert_eq!(32, package_version.id().len());
        assert_eq!(DOCKER_NAMESPACE_ID, package_version.namespace_id());
        assert_eq!("hello-world", package_version.name());
        assert_eq!(PackageTypeName::Docker, *package_version.pkg_type());
        assert_eq!("latest", package_version.version());
        assert!(package_version.license_text().is_none());
        assert!(package_version.license_text_mimetype().is_none());
        assert!(package_version.license_url().is_none());
        assert!(package_version.creation_time().is_none());
        assert!(package_version.modified_time().is_none());
        assert!(package_version.tags().is_empty());
        assert!(package_version.description().is_none());
        assert_eq!(4, package_version.artifacts().len());
        assert_eq!(32, package_version.artifacts()[0].hash().len());
        assert_eq!(
            HashAlgorithm::SHA256,
            *package_version.artifacts()[0].algorithm()
        );
        assert!(package_version.artifacts()[0].name().is_none());
        assert!(package_version.artifacts()[0].creation_time().is_none());
        assert!(package_version.artifacts()[0].url().is_none());
        assert!(package_version.artifacts()[0].size().is_none());
        match package_version.artifacts()[0].mime_type() {
            Some(mime_type) => assert_eq!(MIME_TYPE_BLOB_GZIPPED, mime_type),
            None => assert!(false),
        }
        assert!(package_version.artifacts()[0].metadata().is_empty());
        assert!(package_version.artifacts()[0].source_url().is_none());
        Ok(())
    }
}
