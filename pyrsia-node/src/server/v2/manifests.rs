use std::fs;
use log::debug;
use warp::{Rejection, Reply};
use uuid::Uuid;
use warp::http::{StatusCode};
use bytes::Bytes;
use easy_hasher::easy_hasher::{file_hash, Hash, raw_sha256};
use crate::utils::error_util::RegistryError as registry_err;
use crate::utils::error_util::RegistryErrorCode as registry_err_code;

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
            return Err(warp::reject::custom(registry_err {code: registry_err_code::ManifestUnknown}));
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
        return Err(warp::reject::custom(registry_err {code: registry_err_code::ManifestUnknown}));
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


pub async fn handle_put_manifest(name: String, reference: String, bytes: Bytes) -> Result<impl Reply, Rejection> {
    let id = Uuid::new_v4();

    // temporary upload of manifest
    let blob_upload_dest_dir = format!("/tmp/registry/docker/registry/v2/repositories/{}/_uploads/{}", name, id);
    if let Err(e) = fs::create_dir_all(&blob_upload_dest_dir) {
        return Err(warp::reject::custom(registry_err {code: registry_err_code::Unknown(e.to_string())}));
    }

    let mut blob_upload_dest = format!("/tmp/registry/docker/registry/v2/repositories/{}/_uploads/{}/data", name, id);
    let append = super::blobs::append_to_blob(&mut blob_upload_dest, bytes);
    if let Err(e) = append {
        return Err(warp::reject::custom(registry_err {code: registry_err_code::Unknown(e.to_string())}));
    } else {
        // calculate sha256 checksum on manifest file
        let file256 = file_hash(raw_sha256, &blob_upload_dest);
        let digest: Hash;
        match file256 {
            Ok(hash) => digest = hash,
            Err(e) => return Err(warp::reject::custom(registry_err {code: registry_err_code::Unknown(e.to_string())}))
        }

        let hash = digest.to_hex_string();
        debug!("Generated hash for manifest {}/{}: {}", name, reference, hash);
        let mut blob_dest = format!("/tmp/registry/docker/registry/v2/blobs/sha256/{}/{}", hash.get(0..2).unwrap(), hash);
        if let Err(e) = fs::create_dir_all(&blob_dest) {
            return Err(warp::reject::custom(registry_err {code: registry_err_code::Unknown(e.to_string())}));
        }
        blob_dest.push_str("/data");

        // copy temporary upload to final blob location
        if let Err(e) = fs::copy(&blob_upload_dest, &blob_dest) {
            return Err(warp::reject::custom(registry_err {code: registry_err_code::Unknown(e.to_string())}));
        }

        // remove temporary files
        if let Err(e) = fs::remove_dir_all(blob_upload_dest_dir) {
            return Err(warp::reject::custom(registry_err {code: registry_err_code::Unknown(e.to_string())}));
        }

        // create manifest link file in revisions
        let mut manifest_rev_dest = format!("/tmp/registry/docker/registry/v2/repositories/{}/_manifests/revisions/sha256/{}", name, hash);
        if let Err(e) = fs::create_dir_all(&manifest_rev_dest) {
            return Err(warp::reject::custom(registry_err {code: registry_err_code::Unknown(e.to_string())}));
        }
        manifest_rev_dest.push_str("/link");
        if let Err(e) = fs::write(manifest_rev_dest, format!("sha256:{}", hash)) {
            return Err(warp::reject::custom(registry_err {code: registry_err_code::Unknown(e.to_string())}));
        }

        // create manifest link file in tags if reference is a tag (no colon)
        let colon = reference.find(':');
        if let None = colon {
            let mut manifest_tag_dest = format!("/tmp/registry/docker/registry/v2/repositories/{}/_manifests/tags/{}/current", name, reference);
            if let Err(e) = fs::create_dir_all(&manifest_tag_dest) {
                return Err(warp::reject::custom(registry_err {code: registry_err_code::Unknown(e.to_string())}));
            }
            manifest_tag_dest.push_str("/link");
            if let Err(e) = fs::write(manifest_tag_dest, format!("sha256:{}", hash)) {
                return Err(warp::reject::custom(registry_err {code: registry_err_code::Unknown(e.to_string())}));
            }
        }

        Ok(warp::http::response::Builder::new()
            .header("Location", format!("http://localhost:7878/v2/{}/manifests/sha256:{}", name, hash))
            .header("Docker-Content-Digest", format!("sha256:{}", hash))
            .status(StatusCode::CREATED)
            .body("")
            .unwrap()
        )
    }
}