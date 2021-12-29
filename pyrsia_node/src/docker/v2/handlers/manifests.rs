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

use bytes::Bytes;
use crate::artifact_manager;
use easy_hasher::easy_hasher::{file_hash, raw_sha256, Hash, raw_sha512};
use log::{debug, info, warn};
use std::fs;
use uuid::Uuid;
use warp::http::StatusCode;
use warp::{Rejection, Reply};
use crate::artifact_manager::HashAlgorithm;

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
    return Ok(warp::http::response::Builder::new()
        .header(
            "Content-Type",
            "application/vnd.docker.distribution.manifest.v2+json",
        )
        .header("Content-Length", content.len())
        .status(StatusCode::OK)
        .body(content)
        .unwrap());
}

const LOCATION: &'static str = "Location";

// Handles PUT endpoint documented at https://docs.docker.com/registry/spec/api/#manifest
pub async fn handle_put_manifest(
    name: String,
    reference: String,
    bytes: Bytes,
) -> Result<impl Reply, Rejection> {
    let id = Uuid::new_v4();

    match store_manifest_in_artifact_manager(&bytes) {
        Ok(artifact_hash) => info!("Stored manifest with {} hash {}", artifact_hash.0, artifact_hash),
        Err(error) => warn!("Error storing manifest in artifact_manager {}", error)
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

    let mut blob_upload_dest = format!(
        "/tmp/registry/docker/registry/v2/repositories/{}/_uploads/{}/data",
        name, id
    );
    let append = super::blobs::append_to_blob(&mut blob_upload_dest, bytes);
    if let Err(e) = append {
        return Err(warp::reject::custom(RegistryError {
            code: RegistryErrorCode::Unknown(e.to_string()),
        }));
    } else {
        // calculate sha256 checksum on manifest file
        let file256 = file_hash(raw_sha256, &blob_upload_dest);
        let digest: Hash;
        match file256 {
            Ok(hash) => digest = hash,
            Err(e) => {
                return Err(warp::reject::custom(RegistryError {
                    code: RegistryErrorCode::Unknown(e.to_string()),
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
        if let None = colon {
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
    let mut manifest_vec = bytes.to_vec();
    let sha512: Vec<u8> = raw_sha512(manifest_vec.clone()).to_vec();
    let artifact_hash = artifact_manager::Hash::new(HashAlgorithm::SHA512, &sha512)?;
    crate::node_manager::handlers::ART_MGR.push_artifact(&mut manifest_vec.as_slice(), &artifact_hash)?;
    Ok((HashAlgorithm::SHA512, sha512))
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Context;
    use futures::executor;
    use futures::executor::ThreadPool;
    use serde::de::StdError;
    use std::fs::read_to_string;

    const MANIFEST_JSON: &str = r##"{
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
        let pool = ThreadPool::new().context("Failed to build pool")?;

        let future = async {
            handle_put_manifest(
                name.to_string(),
                reference.to_string(),
                Bytes::from(MANIFEST_JSON.as_bytes()),
            )
            .await
        };
        let result = executor::block_on(future);
        match result {
            Ok(reply) => {
                let response = reply.into_response();
                assert_eq!(response.status(), 201);
                assert!(response.headers().contains_key(LOCATION));
            }
            Err(rejection) => {
                assert!(false)
            }
        };
        Ok(())
    }
}
