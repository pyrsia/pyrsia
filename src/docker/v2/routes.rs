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

use crate::network::client::Client;
use crate::transparency_log::log::TransparencyLog;

use super::handlers::blobs::*;
use super::handlers::manifests::*;
use std::collections::HashMap;
use warp::Filter;

pub fn make_docker_routes(
    transparency_log: TransparencyLog,
    p2p_client: Client,
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

    let p2p_client_get_manifests = p2p_client.clone();
    let p2p_client_put_manifests = p2p_client.clone();
    let transparency_log_get_manifests = transparency_log.clone();

    let v2_manifests = warp::path!("v2" / "library" / String / "manifests" / String)
        .and(warp::get().or(warp::head()).unify())
        .and_then(move |name, tag| {
            fetch_manifest(
                transparency_log_get_manifests.clone(),
                p2p_client_get_manifests.clone(),
                name,
                tag,
            )
        });
    let v2_manifests_put_docker = warp::path!("v2" / "library" / String / "manifests" / String)
        .and(warp::put())
        .and(warp::header::exact(
            "Content-Type",
            "application/vnd.docker.distribution.manifest.v2+json",
        ))
        .and(warp::body::bytes())
        .and_then(move |name, reference, bytes| {
            put_manifest(
                transparency_log.clone(),
                p2p_client_put_manifests.clone(),
                name,
                reference,
                bytes,
            )
        });

    let v2_blobs = warp::path!("v2" / "library" / String / "blobs" / String)
        .and(warp::get().or(warp::head()).unify())
        .and(warp::path::end())
        .and_then(move |name, hash| handle_get_blobs(p2p_client.clone(), name, hash));
    let v2_blobs_post = warp::path!("v2" / "library" / String / "blobs" / "uploads")
        .and(warp::post())
        .and_then(handle_post_blob);
    let v2_blobs_patch = warp::path!("v2" / "library" / String / "blobs" / "uploads" / String)
        .and(warp::patch())
        .and(warp::body::bytes())
        .and_then(handle_patch_blob);
    let v2_blobs_put = warp::path!("v2" / "library" / String / "blobs" / "uploads" / String)
        .and(warp::put())
        .and(warp::query::<HashMap<String, String>>())
        .and(warp::body::bytes())
        .and_then(handle_put_blob);

    warp::any().and(
        v2_base
            .or(v2_manifests)
            .or(v2_manifests_put_docker)
            .or(v2_blobs)
            .or(v2_blobs_post)
            .or(v2_blobs_patch)
            .or(v2_blobs_put),
    )
}
