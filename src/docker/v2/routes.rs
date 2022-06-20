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

use crate::artifact_service::storage::ArtifactStorage;
use crate::docker::v2::handlers::{blobs::handle_get_blobs, manifests::fetch_manifest};
use crate::network::client::Client;
use crate::transparency_log::log::TransparencyLog;
use futures::lock::Mutex;
use std::sync::Arc;
use warp::Filter;

pub fn make_docker_routes(
    transparency_log: TransparencyLog,
    p2p_client: Client,
    artifact_storage: ArtifactStorage,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
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

    let transparency_log_fetch_manifest = Arc::new(Mutex::new(transparency_log));
    let transparency_log_get_blobs = transparency_log_fetch_manifest.clone();
    let p2p_client_fetch_manifest = p2p_client.clone();
    let artifact_storage_fetch_manifest = artifact_storage.clone();

    let v2_manifests = warp::path!("v2" / "library" / String / "manifests" / String)
        .and(warp::get().or(warp::head()).unify())
        .and_then(move |name, tag| {
            fetch_manifest(
                transparency_log_fetch_manifest.clone(),
                p2p_client_fetch_manifest.clone(),
                artifact_storage_fetch_manifest.clone(),
                name,
                tag,
            )
        });

    let v2_blobs = warp::path!("v2" / "library" / String / "blobs" / String)
        .and(warp::get().or(warp::head()).unify())
        .and(warp::path::end())
        .and_then(move |_name, hash| {
            handle_get_blobs(
                transparency_log_get_blobs.clone(),
                p2p_client.clone(),
                artifact_storage.clone(),
                hash,
            )
        });

    warp::any().and(v2_base.or(v2_manifests).or(v2_blobs))
}
