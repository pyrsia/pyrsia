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

use super::{RegistryError, RegistryErrorCode};

use crate::artifact_manager;
use crate::artifact_manager::HashAlgorithm;
use crate::node_manager::model::artifact::Artifact;
use crate::node_manager::model::package_version::PackageVersion;
use anyhow::{anyhow, Context, Error};
use bytes::Bytes;
use easy_hasher::easy_hasher::{file_hash, raw_sha256, raw_sha512, Hash};
use log::{debug, error, info, warn};
use serde_json::{Map, Value};
use std::fs;
use uuid::Uuid;
use warp::http::StatusCode;
use warp::{Rejection, Reply};

// Handles GET endpoint documented at https://docs.docker.com/registry/spec/api/#manifest
pub async fn handle_get_manifests(name: String, tag: String) -> Result<impl Reply, Rejection> {
    let colon = tag.find(':');
    let mut hash = String::from(&tag);
    if colon == None {
        let manifest = format!(
            "/tmp/registry/docker/registry/v2/repositories/{}/_manifests/tags/{}/current/link",
            name, tag
        );
        let manifest_content = fs::read_to_string(manifest);
        if manifest_content.is_err() {
            return Err(warp::reject::custom(RegistryError {
                code: RegistryErrorCode::ManifestUnknown,
            }));
        }
        hash = manifest_content.unwrap();
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
    let id = Uuid::new_v4();

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

    // temporary upload of manifest
    let blob_upload_dest_dir = format!(
        "/tmp/registry/docker/registry/v2/repositories/{}/_uploads/{}",
        name, id
    );
    if let Err(e) = fs::create_dir_all(&blob_upload_dest_dir) {
        return Err(warp::reject::custom(RegistryError {
            code: RegistryErrorCode::Unknown(e.to_string()),
        }));
    }

    let blob_upload_dest = format!(
        "/tmp/registry/docker/registry/v2/repositories/{}/_uploads/{}/data",
        name, id
    );
    let append = super::blobs::append_to_blob(&blob_upload_dest, bytes);
    if let Err(e) = append {
        Err(warp::reject::custom(RegistryError {
            code: RegistryErrorCode::Unknown(e.to_string()),
        }))
    } else {
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
        if let Err(e) = fs::create_dir_all(&blob_dest) {
            return Err(warp::reject::custom(RegistryError {
                code: RegistryErrorCode::Unknown(e.to_string()),
            }));
        }
        blob_dest.push_str("/data");

        // copy temporary upload to final blob location
        if let Err(e) = fs::copy(&blob_upload_dest, &blob_dest) {
            return Err(warp::reject::custom(RegistryError {
                code: RegistryErrorCode::Unknown(e.to_string()),
            }));
        }

        // remove temporary files
        if let Err(e) = fs::remove_dir_all(blob_upload_dest_dir) {
            return Err(warp::reject::custom(RegistryError {
                code: RegistryErrorCode::Unknown(e.to_string()),
            }));
        }

        // create manifest link file in revisions
        let mut manifest_rev_dest = format!(
            "/tmp/registry/docker/registry/v2/repositories/{}/_manifests/revisions/sha256/{}",
            name, hash
        );
        if let Err(e) = fs::create_dir_all(&manifest_rev_dest) {
            return Err(warp::reject::custom(RegistryError {
                code: RegistryErrorCode::Unknown(e.to_string()),
            }));
        }
        manifest_rev_dest.push_str("/link");
        if let Err(e) = fs::write(manifest_rev_dest, format!("sha256:{}", hash)) {
            return Err(warp::reject::custom(RegistryError {
                code: RegistryErrorCode::Unknown(e.to_string()),
            }));
        }

        // create manifest link file in tags if reference is a tag (no colon)
        let colon = reference.find(':');
        if colon.is_none() {
            let mut manifest_tag_dest = format!(
                "/tmp/registry/docker/registry/v2/repositories/{}/_manifests/tags/{}/current",
                name, reference
            );
            if let Err(e) = fs::create_dir_all(&manifest_tag_dest) {
                return Err(warp::reject::custom(RegistryError {
                    code: RegistryErrorCode::Unknown(e.to_string()),
                }));
            }
            manifest_tag_dest.push_str("/link");
            if let Err(e) = fs::write(manifest_tag_dest, format!("sha256:{}", hash)) {
                return Err(warp::reject::custom(RegistryError {
                    code: RegistryErrorCode::Unknown(e.to_string()),
                }));
            }
        }

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
}

fn store_manifest_in_artifact_manager(bytes: &Bytes) -> anyhow::Result<(HashAlgorithm, Vec<u8>)> {
    let manifest_vec = bytes.to_vec();
    let sha512: Vec<u8> = raw_sha512(manifest_vec.clone()).to_vec();
    let artifact_hash = artifact_manager::Hash::new(HashAlgorithm::SHA512, &sha512)?;
    crate::node_manager::handlers::ART_MGR
        .push_artifact(&mut manifest_vec.as_slice(), &artifact_hash)?;
    Ok((HashAlgorithm::SHA512, sha512))
}

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
        Err(_) => Err(anyhow!(
            "Error parsing docker manifest JSON: {}",
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
    match manifest_schema_version(json_object, json_string)? {
        1 => package_version_from_schema1(&json_object),
        2 => package_version_from_schema2(&json_object, json_string, docker_name, docker_reference),
        n => Err(anyhow!("Unsupported manifest schema version {}", n)),
    }
}

fn package_version_from_schema1(
    json_object: &Map<String, Value>,
) -> Result<PackageVersion, anyhow::Error> {
    let manifest_name = value_of(json_object, "name", serde_json::value::Value::as_str)?;
    let manifest_tag = value_of(json_object, "tag", serde_json::value::Value::as_str)?;
    let fslayers = json_object
        .get("fslayers")
        .context("missing fslayers field")?
        .as_array()
        .context("invalid fslayers")?;
    let mut artifacts: Vec<Artifact> = Vec::new();
    for fslayer in fslayers {
        let hex_digest = fslayer
            .as_object()
            .context("invalid fslayer")?
            .get("blobSum")
            .context("missing blobSum")?
            .as_str()
            .context("invalid blogSum")?;
        if !hex_digest.starts_with("sha256:") {
            return Err(anyhow!("Only sha256 digests are supported: {}", hex_digest));
        }
        let digest = hex::decode(&hex_digest["sha256:".len()..])?;
        artifacts.push( Artifact::new(digest, HashAlgorithm::SHA256, None, None, None, None, None, Map::new(), None))
    }
    Ok(PackageVersion::new())
}

fn package_version_from_schema2(
    json_object: &Map<String, Value>,
    json_string: &str,
    docker_name: &str,
    docker_reference: &str,
) -> Result<PackageVersion, anyhow::Error> {
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
            .with_context(|| !format!("Invalid schemaVersion value: {}", s)),
        _ => invalid_manifest(json_string),
    }
}

fn value_of<T>(
    json_object: &Map<String, Value>,
    field_name: &str,
    f: fn(&Value) -> Option<T>,
) -> Result<T, anyhow::Error> {
    f(json_object
        .get(field_name)
        .with_context(|| format!("missing {} field", field_name))?)
    .with_context(|| format!("invalid {}", field_name))
}

fn invalid_manifest<T>(json_string: &str) -> Result<T, anyhow::Error> {
    Err(anyhow!("Invalid JSON manifest: {}", json_string))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_manager::handlers::ART_MGR;
    use anyhow::Context;
    use futures::executor;
    use futures::executor::ThreadPool;
    use serde::de::StdError;
    use std::fs::read_to_string;

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
      },
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

    const MANIFEST_V2_IMAGE_JSON: &str = r##"{
	"schemaVersion": 2,
	"mediaType": "application/vnd.docker.distribution.manifest.v2+json",
	"config": {
		"mediaType": "application/vnd.docker.container.image.v1+json",
		"size": 5215,
		"digest": "sha256:b138b9264903f46a43e1c750e07dc06f5d2a1bd5d51f37fb185bc608f61090dd"
	},
	"layers": [
		{
			"mediaType": "application/vnd.docker.image.rootfs.diff.tar.gzip",
			"size": 32034160,
			"digest": "sha256:473ede7ed136b710ab2dd51579af038b7d00fbbf6a1790c6294c93666203c0a6"
		},
		{
			"mediaType": "application/vnd.docker.image.rootfs.diff.tar.gzip",
			"size": 843,
			"digest": "sha256:c46b5fa4d940569e49988515c1ea0295f56d0a16228d8f854e27613f467ec892"
		},
		{
			"mediaType": "application/vnd.docker.image.rootfs.diff.tar.gzip",
			"size": 554,
			"digest": "sha256:93ae3df89c92cb1d20e9c09f499e693d3a8a8cef161f7158f7a9a3b5d06e4ef2"
		},
		{
			"mediaType": "application/vnd.docker.image.rootfs.diff.tar.gzip",
			"size": 162,
			"digest": "sha256:6b1eed27cadec5de8051d56697b0b67527e4076deedceefb41b7b2ea9b900459"
		},
		{
			"mediaType": "application/vnd.docker.image.rootfs.diff.tar.gzip",
			"size": 169218938,
			"digest": "sha256:0373952b589d2d14782a35c2e67826e80c814e5d3ae41370a6dc89ed43c2e60b"
		},
		{
			"mediaType": "application/vnd.docker.image.rootfs.diff.tar.gzip",
			"size": 108979,
			"digest": "sha256:7b82cd0ee5279a665a15cb61719276284e769e4b980f46709b21e53183974eec"
		},
		{
			"mediaType": "application/vnd.docker.image.rootfs.diff.tar.gzip",
			"size": 12803584,
			"digest": "sha256:a36b2d884a8941918fba8ffd1599b5187de99bd30c8aa112694fc5f8d024f506"
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
}
