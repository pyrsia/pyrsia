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

use crate::docker::error_util::{RegistryError, RegistryErrorCode};
use crate::metadata_manager::metadata::MetadataCreationStatus;
use crate::node_manager::handlers::{put_artifact, METADATA_MGR};
use crate::node_manager::model::artifact::{Artifact, ArtifactBuilder};
use crate::node_manager::model::package_type::PackageTypeName;
use crate::node_manager::model::package_version::PackageVersion;
use crate::node_manager::model::DOCKER_NAMESPACE_ID;
use crate::node_manager::HashAlgorithm;

use anyhow::{anyhow, bail, Context};
use bytes::{Buf, Bytes};
use easy_hasher::easy_hasher::raw_sha512;
use log::{debug, error, info, warn};
use serde_json::{json, Map, Value};
use uuid::Uuid;
use warp::Rejection;

pub fn get_artifact_manifest(artifacts: &[Artifact]) -> Option<&Artifact> {
    for artifact in artifacts {
        if let Some(mime_type) = artifact.mime_type() {
            if mime_type.eq(MEDIA_TYPE_SCHEMA_1)
                || mime_type.eq(MEDIA_TYPE_IMAGE_MANIFEST)
                || mime_type.eq(MEDIA_TYPE_MANIFEST_LIST)
            {
                return Some(artifact);
            }
        }
    }
    None
}

pub fn store_manifest_in_artifact_manager(
    name: &str,
    tag: &str,
    bytes: &Bytes,
) -> Result<String, Rejection> {
    let manifest_vec = bytes.to_vec();
    let sha512: Vec<u8> = raw_sha512(manifest_vec).to_vec();
    match put_artifact(
        &sha512,
        Box::new(bytes.clone().reader()),
        HashAlgorithm::SHA512,
    ) {
        Ok(_) => {
            let hash = hex::encode(&sha512);
            info!(
                "Stored manifest with {} hash {}",
                HashAlgorithm::SHA512,
                &hash
            );
            let mut package_version = match package_version_from_manifest_bytes(
                bytes,
                name,
                tag,
                HashAlgorithm::SHA512,
                sha512,
            ) {
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
            if let Err(err) = save_package_version(&mut package_version) {
                return Err(warp::reject::custom(RegistryError {
                    code: RegistryErrorCode::Unknown(err.to_string()),
                }));
            };

            return Ok(hash);
        }
        Err(error) => warn!("Error storing manifest in artifact_manager {}", error),
    };

    Ok(String::from(""))
}

fn save_package_version(package_version: &mut PackageVersion) -> Result<(), anyhow::Error> {
    let pv_json = serde_json::to_string(package_version)
        .unwrap_or_else(|_| "*** missing JSON ***".to_string());
    match METADATA_MGR.create_package_version(package_version)? {
        MetadataCreationStatus::Created => {
            info!("Saved package version from docker manifest: {}", pv_json)
        }
        MetadataCreationStatus::Duplicate { json } => info!(
            "Package version from docker manifest {}\nwas a duplicate of previously stored {}",
            pv_json, json
        ),
    };
    Ok(())
}

fn package_version_from_manifest_bytes(
    bytes: &Bytes,
    docker_name: &str,
    docker_reference: &str,
    hash_algorithm: HashAlgorithm,
    hash: Vec<u8>,
) -> Result<PackageVersion, anyhow::Error> {
    let json_string = String::from_utf8(bytes.to_vec())?;
    match serde_json::from_str::<Value>(&json_string) {
        Ok(Value::Object(json_object)) => {
            let result = package_version_from_manifest_json(
                &json_object,
                &json_string,
                docker_name,
                docker_reference,
                hash_algorithm,
                hash,
                bytes.len(),
            );
            if result.is_err() {
                error!("Invalid manifest {}", json_string)
            }
            result
        }
        Ok(_) => invalid_manifest(&json_string),
        Err(err) => Err(anyhow!(
            "Error parsing docker manifest JSON: {} in {}",
            err,
            json_string
        )),
    }
}

fn package_version_from_manifest_json(
    json_object: &Map<String, Value>,
    json_string: &str,
    docker_name: &str,
    docker_reference: &str,
    hash_algorithm: HashAlgorithm,
    hash: Vec<u8>,
    size: usize,
) -> Result<PackageVersion, anyhow::Error> {
    match manifest_schema_version(json_object, json_string)? {
        1 => package_version_from_schema1(json_object, hash_algorithm, hash, size),
        2 => package_version_from_schema2(
            json_object,
            docker_name,
            docker_reference,
            hash_algorithm,
            hash,
            size,
        ),
        n => Err(anyhow!("Unsupported manifest schema version {}", n)),
    }
}

const CONFIG: &str = "config";
const DIGEST: &str = "digest";
const FS_LAYERS: &str = "fsLayers";
const LAYERS: &str = "layers";
const MANIFESTS: &str = "manifests";
const MEDIA_TYPE: &str = "mediaType";
const SIZE: &str = "size";

const MEDIA_TYPE_BLOB_GZIPPED: &str = "application/vnd.docker.image.rootfs.diff.tar.gzip";
const MEDIA_TYPE_SCHEMA_1: &str = "application/vnd.docker.distribution.manifest.v1+json";
const MEDIA_TYPE_IMAGE_MANIFEST: &str = "application/vnd.docker.distribution.manifest.v2+json";
const MEDIA_TYPE_MANIFEST_LIST: &str = "application/vnd.docker.distribution.manifest.list.v2+json";

fn package_version_from_schema1(
    json_object: &Map<String, Value>,
    hash_algorithm: HashAlgorithm,
    hash: Vec<u8>,
    size: usize,
) -> Result<PackageVersion, anyhow::Error> {
    debug!("Processing schema 1 manifest");
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
    let mut metadata = Map::new();
    metadata.insert(MEDIA_TYPE.to_string(), json!(MEDIA_TYPE_SCHEMA_1));
    let mut artifacts: Vec<Artifact> = Vec::new();
    let size64 = u64::try_from(size)?;
    artifacts.push(
        ArtifactBuilder::default()
            .algorithm(hash_algorithm)
            .hash(hash)
            .mime_type(MEDIA_TYPE_SCHEMA_1.to_string())
            .size(size64)
            .build()?,
    );
    for fslayer in fslayers {
        add_fslayers(&mut artifacts, fslayer)?;
    }
    build_package_version(manifest_name, manifest_tag, metadata, artifacts)
}

fn build_package_version(
    manifest_name: &str,
    manifest_tag: &str,
    metadata: Map<String, Value>,
    artifacts: Vec<Artifact>,
) -> anyhow::Result<PackageVersion> {
    Ok(PackageVersion::new(
        new_uuid_string(),
        DOCKER_NAMESPACE_ID.to_string(),
        String::from(manifest_name),
        PackageTypeName::Docker,
        metadata,
        String::from(manifest_tag),
        artifacts,
    ))
}

fn add_fslayers(artifacts: &mut Vec<Artifact>, fslayer: &Value) -> Result<(), anyhow::Error> {
    let hex_digest = fslayer
        .as_object()
        .context("invalid fslayer")?
        .get("blobSum")
        .context("missing blobSum")?
        .as_str()
        .context("invalid blobSum")?;
    let digest = extract_digest(hex_digest)?;
    artifacts.push(
        ArtifactBuilder::default()
            .algorithm(HashAlgorithm::SHA256)
            .hash(digest)
            .mime_type(MEDIA_TYPE_BLOB_GZIPPED.to_string())
            .build()?,
    );
    Ok(())
}

fn extract_digest(hex_digest: &str) -> Result<Vec<u8>, anyhow::Error> {
    if !hex_digest.starts_with("sha256:") {
        return Err(anyhow!("Only sha256 digests are supported: {}", hex_digest));
    }
    hex::decode(&hex_digest["sha256:".len()..])
        .context(format!("Badly formatted digest: {}", hex_digest))
}

fn package_version_from_schema2(
    json_object: &Map<String, Value>,
    docker_name: &str,
    docker_reference: &str,
    hash_algorithm: HashAlgorithm,
    hash: Vec<u8>,
    size: usize,
) -> Result<PackageVersion, anyhow::Error> {
    debug!("Processing schema version 2 manifest");
    let manifest_media_type = json_object
        .get(MEDIA_TYPE)
        .context("Missing mediaType")?
        .as_str()
        .context("Invalid mediaType")?;
    match manifest_media_type {
        MEDIA_TYPE_IMAGE_MANIFEST => package_version_from_image_manifest(
            json_object,
            docker_name,
            docker_reference,
            hash_algorithm,
            hash,
            size,
        ),
        MEDIA_TYPE_MANIFEST_LIST => package_version_from_manifest_list(
            json_object,
            docker_name,
            docker_reference,
            hash_algorithm,
            hash,
            size,
        ),
        _ => bail!("Manifest has unknown media type: {}", manifest_media_type),
    }
}

fn package_version_from_manifest_list(
    json_object: &Map<String, Value>,
    docker_name: &str,
    docker_reference: &str,
    hash_algorithm: HashAlgorithm,
    hash: Vec<u8>,
    size: usize,
) -> Result<PackageVersion, anyhow::Error> {
    debug!("Processing manifest list");
    let mut metadata = Map::new();
    metadata.insert(MEDIA_TYPE.to_string(), json!(MEDIA_TYPE_MANIFEST_LIST));
    let mut artifacts: Vec<Artifact> = Vec::new();
    let size64 = u64::try_from(size)?;
    artifacts.push(
        ArtifactBuilder::default()
            .algorithm(hash_algorithm)
            .hash(hash)
            .mime_type(MEDIA_TYPE_MANIFEST_LIST.to_string())
            .size(size64)
            .build()?,
    );
    let manifests = json_object
        .get(MANIFESTS)
        .context("Manifest list has no manifests field")?
        .as_array()
        .context("Value of manifests field is not an array")?;
    for manifest in manifests {
        add_artifact(&mut artifacts, manifest, "manifest")?
    }
    build_package_version(docker_name, docker_reference, metadata, artifacts)
}

fn package_version_from_image_manifest(
    json_object: &Map<String, Value>,
    docker_name: &str,
    docker_reference: &str,
    hash_algorithm: HashAlgorithm,
    hash: Vec<u8>,
    size: usize,
) -> Result<PackageVersion, anyhow::Error> {
    debug!("Processing image manifest");
    let mut metadata = Map::new();
    metadata.insert(MEDIA_TYPE.to_string(), json!(MEDIA_TYPE_IMAGE_MANIFEST));
    let mut artifacts: Vec<Artifact> = Vec::new();
    let size64 = u64::try_from(size)?;
    artifacts.push(
        ArtifactBuilder::default()
            .algorithm(hash_algorithm)
            .hash(hash)
            .mime_type(MEDIA_TYPE_IMAGE_MANIFEST.to_string())
            .size(size64)
            .build()?,
    );
    if let Some(config) = json_object.get(CONFIG) {
        add_artifact(&mut artifacts, config, "config")?
    }
    let layers = json_object
        .get(LAYERS)
        .context("Image manifest has no layers field")?
        .as_array()
        .context("Value of layers field is not an array")?;
    for layer in layers {
        add_artifact(&mut artifacts, layer, "layer")?
    }
    build_package_version(docker_name, docker_reference, metadata, artifacts)
}

fn add_artifact(
    artifacts: &mut Vec<Artifact>,
    json_object: &Value,
    name: &str,
) -> Result<(), anyhow::Error> {
    artifacts.push(
        ArtifactBuilder::default()
            .algorithm(HashAlgorithm::SHA256)
            .hash(extract_digest(
                json_object
                    .get(DIGEST)
                    .with_context(|| format!("{} is missing digest", name))?
                    .as_str()
                    .with_context(|| format!("{} has invalid digest", name))?,
            )?)
            .size(
                json_object
                    .get(SIZE)
                    .with_context(|| format!("{} is missing size", name))?
                    .as_u64()
                    .with_context(|| format!("{} has invalid size", name))?,
            )
            .mime_type(
                json_object
                    .get(MEDIA_TYPE)
                    .with_context(|| format!("{} is missing mediaType", name))?
                    .as_str()
                    .with_context(|| format!("{} has invalid mediaType", name))?
                    .to_string(),
            )
            .build()?,
    );
    Ok(())
}

fn new_uuid_string() -> String {
    String::from(
        Uuid::new_v4()
            .to_simple()
            .encode_lower(&mut Uuid::encode_buffer()),
    )
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

fn invalid_manifest<T>(_json_string: &str) -> Result<T, anyhow::Error> {
    Err(anyhow!("Invalid JSON manifest: {}", _json_string))
}

#[cfg(test)]
pub mod tests {
    use super::*;

    const MEDIA_TYPE_CONFIG_JSON: &str = "application/vnd.docker.container.image.v1+json";

    pub const MANIFEST_V1_JSON: &str = r##"{
        "name": "hello-world",
        "tag": "v3.1",
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
            }
        ]
    }"##;

    const MANIFEST_V2_IMAGE: &str = r##"{
        "schemaVersion": 2,
        "mediaType": "application/vnd.docker.distribution.manifest.v2+json",
        "config": {
            "mediaType": "application/vnd.docker.container.image.v1+json",
            "size": 7023,
            "digest": "sha256:b5b2b2c507a0944348e0303114d8d93aaaa081732b86451d9bce1f432a537bc7"
        },
        "layers": [
            {
                "mediaType": "application/vnd.docker.image.rootfs.diff.tar.gzip",
                "size": 32654,
                "digest": "sha256:e692418e4cbaf90ca69d05a66403747baa33ee08806650b51fab815ad7fc331f"
            },
            {
                "mediaType": "application/vnd.docker.image.rootfs.diff.tar.gzip",
                "size": 16724,
                "digest": "sha256:3c3a4604a545cdc127456d94e421cd355bca5b528f4a9c1905b15da2eb4a4c6b"
            },
            {
                "mediaType": "application/vnd.docker.image.rootfs.diff.tar.gzip",
                "size": 73109,
                "digest": "sha256:ec4b8955958665577945c89419d1af06b5f7636b4ac3da7f12184802ad867736"
            }
        ]
    }"##;

    const MANIFEST_V2_LIST: &str = r##"{
        "schemaVersion": 2,
        "mediaType": "application/vnd.docker.distribution.manifest.list.v2+json",
        "manifests": [
            {
                "mediaType": "application/vnd.docker.distribution.manifest.v2+json",
                "size": 7143,
                "digest": "sha256:e692418e4cbaf90ca69d05a66403747baa33ee08806650b51fab815ad7fc331f",
                "platform": {
                    "architecture": "ppc64le",
                    "os": "linux"
                }
            },
            {
                "mediaType": "application/vnd.docker.distribution.manifest.v2+json",
                "size": 7682,
                "digest": "sha256:5b0bcabd1ed22e9fb1310cf6c2dec7cdef19f0ad69efa1f392e94a4333501270",
                "platform": {
                    "architecture": "amd64",
                    "os": "linux",
                    "features": [
                        "sse4"
                    ]
                }
            }
        ]
    }"##;

    #[test]
    fn test_extraction_of_package_version_from_manifest_conforming_to_schema_version_1(
    ) -> Result<(), anyhow::Error> {
        let json_bytes = Bytes::from(MANIFEST_V1_JSON);
        let hash: Vec<u8> = raw_sha512(json_bytes.to_vec()).to_vec();
        let package_version: PackageVersion = package_version_from_manifest_bytes(
            &json_bytes,
            "test_pkg",
            "v1.4",
            HashAlgorithm::SHA512,
            hash.clone(),
        )?;
        assert_eq!(32, package_version.id.len());
        assert_eq!(DOCKER_NAMESPACE_ID, package_version.namespace_id);
        assert_eq!("hello-world", package_version.name);
        assert_eq!(PackageTypeName::Docker, package_version.pkg_type);
        assert_eq!("v3.1", package_version.version);
        assert!(package_version.license_text.is_none());
        assert!(package_version.license_text_mimetype.is_none());
        assert!(package_version.license_url.is_none());
        assert!(package_version.creation_time.is_none());
        assert!(package_version.modified_time.is_none());
        assert!(package_version.tags.is_empty());
        assert!(package_version.metadata.contains_key(MEDIA_TYPE));
        assert_eq!(
            MEDIA_TYPE_SCHEMA_1,
            package_version.metadata[MEDIA_TYPE].as_str().unwrap()
        );
        assert!(package_version.description.is_none());
        assert_eq!(5, package_version.artifacts.len());

        assert_eq!(64, package_version.artifacts[0].hash().len());
        assert_eq!(&hash, package_version.artifacts[0].hash());
        assert_eq!(
            HashAlgorithm::SHA512,
            *package_version.artifacts[0].algorithm()
        );
        assert!(package_version.artifacts[0].name().is_none());
        assert!(package_version.artifacts[0].creation_time().is_none());
        assert!(package_version.artifacts[0].url().is_none());
        assert_eq!(
            u64::try_from(MANIFEST_V1_JSON.len())?,
            package_version.artifacts[0].size().unwrap()
        );
        match package_version.artifacts[0].mime_type() {
            Some(mime_type) => assert_eq!(MEDIA_TYPE_SCHEMA_1, mime_type),
            None => assert!(false),
        }
        assert!(package_version.artifacts[0].metadata().is_empty());
        assert!(package_version.artifacts[0].source_url().is_none());

        assert!(package_version.artifacts[1].name().is_none());
        assert!(package_version.artifacts[1].creation_time().is_none());
        assert!(package_version.artifacts[1].url().is_none());
        assert!(package_version.artifacts[1].size().is_none());
        assert_eq!(
            HashAlgorithm::SHA256,
            *package_version.artifacts[1].algorithm()
        );
        assert_eq!(
            &vec![
                0x5fu8, 0x70u8, 0xbfu8, 0x18u8, 0xa0u8, 0x86u8, 0x00u8, 0x70u8, 0x16u8, 0xe9u8,
                0x48u8, 0xb0u8, 0x4au8, 0xedu8, 0x3bu8, 0x82u8, 0x10u8, 0x3au8, 0x36u8, 0xbeu8,
                0xa4u8, 0x17u8, 0x55u8, 0xb6u8, 0xcdu8, 0xdfu8, 0xafu8, 0x10u8, 0xacu8, 0xe3u8,
                0xc6u8, 0xefu8
            ],
            package_version.artifacts[1].hash()
        );

        //
        match package_version.artifacts[1].mime_type() {
            Some(mime_type) => assert_eq!(MEDIA_TYPE_BLOB_GZIPPED, mime_type),
            None => assert!(false),
        }
        assert!(package_version.artifacts[1].metadata().is_empty());
        assert!(package_version.artifacts[1].source_url().is_none());
        Ok(())
    }

    #[test]
    fn test_extraction_of_package_version_from_image_manifest() -> Result<(), anyhow::Error> {
        let json_bytes = Bytes::from(MANIFEST_V2_IMAGE);
        let hash: Vec<u8> = raw_sha512(json_bytes.to_vec()).to_vec();
        let package_version: PackageVersion = package_version_from_manifest_bytes(
            &json_bytes,
            "test_pkg",
            "v1.4",
            HashAlgorithm::SHA512,
            hash.clone(),
        )?;
        assert_eq!(32, package_version.id.len());
        assert_eq!(DOCKER_NAMESPACE_ID, package_version.namespace_id);
        assert_eq!("test_pkg", package_version.name);
        assert_eq!(PackageTypeName::Docker, package_version.pkg_type);
        assert_eq!("v1.4", package_version.version);
        assert!(package_version.license_text.is_none());
        assert!(package_version.license_text_mimetype.is_none());
        assert!(package_version.license_url.is_none());
        assert!(package_version.creation_time.is_none());
        assert!(package_version.modified_time.is_none());
        assert!(package_version.tags.is_empty());
        assert!(package_version.metadata.contains_key(MEDIA_TYPE));
        assert_eq!(
            MEDIA_TYPE_IMAGE_MANIFEST,
            package_version.metadata[MEDIA_TYPE].as_str().unwrap()
        );
        assert!(package_version.description.is_none());
        assert_eq!(5, package_version.artifacts.len());

        assert_eq!(&hash, package_version.artifacts[0].hash());
        assert_eq!(
            HashAlgorithm::SHA512,
            *package_version.artifacts[0].algorithm()
        );
        assert!(package_version.artifacts[0].name().is_none());
        assert!(package_version.artifacts[0].creation_time().is_none());
        assert!(package_version.artifacts[0].url().is_none());
        assert_eq!(
            u64::try_from(MANIFEST_V2_IMAGE.len())?,
            package_version.artifacts[0].size().unwrap()
        );
        match package_version.artifacts[0].mime_type() {
            Some(mime_type) => assert_eq!(MEDIA_TYPE_IMAGE_MANIFEST, mime_type),
            None => assert!(false),
        }
        assert!(package_version.artifacts[0].metadata().is_empty());
        assert!(package_version.artifacts[0].source_url().is_none());

        assert!(package_version.artifacts[1].name().is_none());
        assert!(package_version.artifacts[1].creation_time().is_none());
        assert!(package_version.artifacts[1].url().is_none());
        assert_eq!(7023u64, package_version.artifacts[1].size().unwrap());
        match package_version.artifacts[1].mime_type() {
            Some(mime_type) => assert_eq!(MEDIA_TYPE_CONFIG_JSON, mime_type),
            None => assert!(false),
        }
        assert!(package_version.artifacts[1].metadata().is_empty());
        assert!(package_version.artifacts[1].source_url().is_none());
        assert_eq!(
            HashAlgorithm::SHA256,
            *package_version.artifacts[1].algorithm()
        );
        assert_eq!(
            &vec![
                0xb5u8, 0xb2u8, 0xb2u8, 0xc5u8, 0x07u8, 0xa0u8, 0x94u8, 0x43u8, 0x48u8, 0xe0u8,
                0x30u8, 0x31u8, 0x14u8, 0xd8u8, 0xd9u8, 0x3au8, 0xaau8, 0xa0u8, 0x81u8, 0x73u8,
                0x2bu8, 0x86u8, 0x45u8, 0x1du8, 0x9bu8, 0xceu8, 0x1fu8, 0x43u8, 0x2au8, 0x53u8,
                0x7bu8, 0xc7u8
            ],
            package_version.artifacts[1].hash()
        );

        assert!(package_version.artifacts[2].name().is_none());
        assert!(package_version.artifacts[2].creation_time().is_none());
        assert!(package_version.artifacts[2].url().is_none());
        assert_eq!(32654u64, package_version.artifacts[2].size().unwrap());
        match package_version.artifacts[2].mime_type() {
            Some(mime_type) => assert_eq!(MEDIA_TYPE_BLOB_GZIPPED, mime_type),
            None => assert!(false),
        }
        assert!(package_version.artifacts[2].metadata().is_empty());
        assert!(package_version.artifacts[2].source_url().is_none());
        assert_eq!(
            HashAlgorithm::SHA256,
            *package_version.artifacts[2].algorithm()
        );
        assert_eq!(
            &vec![
                0xe6u8, 0x92u8, 0x41u8, 0x8eu8, 0x4cu8, 0xbau8, 0xf9u8, 0x0cu8, 0xa6u8, 0x9du8,
                0x05u8, 0xa6u8, 0x64u8, 0x03u8, 0x74u8, 0x7bu8, 0xaau8, 0x33u8, 0xeeu8, 0x08u8,
                0x80u8, 0x66u8, 0x50u8, 0xb5u8, 0x1fu8, 0xabu8, 0x81u8, 0x5au8, 0xd7u8, 0xfcu8,
                0x33u8, 0x1fu8
            ],
            package_version.artifacts[2].hash()
        );

        Ok(())
    }

    #[test]
    fn test_extraction_of_package_version_from_manifest_list() -> Result<(), anyhow::Error> {
        let json_bytes = Bytes::from(MANIFEST_V2_LIST);
        let hash: Vec<u8> = raw_sha512(json_bytes.to_vec()).to_vec();
        let package_version: PackageVersion = package_version_from_manifest_bytes(
            &json_bytes,
            "test_impls",
            "v1.5.2",
            HashAlgorithm::SHA512,
            hash.clone(),
        )?;
        assert_eq!(32, package_version.id.len());
        assert_eq!(DOCKER_NAMESPACE_ID, package_version.namespace_id);
        assert_eq!("test_impls", package_version.name);
        assert_eq!(PackageTypeName::Docker, package_version.pkg_type);
        assert_eq!("v1.5.2", package_version.version);
        assert!(package_version.license_text.is_none());
        assert!(package_version.license_text_mimetype.is_none());
        assert!(package_version.license_url.is_none());
        assert!(package_version.creation_time.is_none());
        assert!(package_version.modified_time.is_none());
        assert!(package_version.tags.is_empty());
        assert!(package_version.metadata.contains_key(MEDIA_TYPE));
        assert_eq!(
            MEDIA_TYPE_MANIFEST_LIST,
            package_version.metadata[MEDIA_TYPE].as_str().unwrap()
        );
        assert!(package_version.description.is_none());
        assert_eq!(3, package_version.artifacts.len());

        assert_eq!(&hash, package_version.artifacts[0].hash());
        assert_eq!(
            HashAlgorithm::SHA512,
            *package_version.artifacts[0].algorithm()
        );
        assert!(package_version.artifacts[0].name().is_none());
        assert!(package_version.artifacts[0].creation_time().is_none());
        assert!(package_version.artifacts[0].url().is_none());
        assert_eq!(
            u64::try_from(MANIFEST_V2_LIST.len())?,
            package_version.artifacts[0].size().unwrap()
        );
        match package_version.artifacts[0].mime_type() {
            Some(mime_type) => assert_eq!(MEDIA_TYPE_MANIFEST_LIST, mime_type),
            None => assert!(false),
        }
        assert!(package_version.artifacts[0].metadata().is_empty());
        assert!(package_version.artifacts[0].source_url().is_none());

        assert!(package_version.artifacts[1].name().is_none());
        assert!(package_version.artifacts[1].creation_time().is_none());
        assert!(package_version.artifacts[1].url().is_none());
        assert_eq!(7143u64, package_version.artifacts[1].size().unwrap());
        match package_version.artifacts[1].mime_type() {
            Some(mime_type) => assert_eq!(MEDIA_TYPE_IMAGE_MANIFEST, mime_type),
            None => assert!(false),
        }
        assert!(package_version.artifacts[1].metadata().is_empty());
        assert!(package_version.artifacts[1].source_url().is_none());
        assert_eq!(
            HashAlgorithm::SHA256,
            *package_version.artifacts[1].algorithm()
        );
        assert_eq!(
            &vec![
                0xe6u8, 0x92u8, 0x41u8, 0x8eu8, 0x4cu8, 0xbau8, 0xf9u8, 0x0cu8, 0xa6u8, 0x9du8,
                0x05u8, 0xa6u8, 0x64u8, 0x03u8, 0x74u8, 0x7bu8, 0xaau8, 0x33u8, 0xeeu8, 0x08u8,
                0x80u8, 0x66u8, 0x50u8, 0xb5u8, 0x1fu8, 0xabu8, 0x81u8, 0x5au8, 0xd7u8, 0xfcu8,
                0x33u8, 0x1fu8
            ],
            package_version.artifacts[1].hash()
        );

        Ok(())
    }
}
