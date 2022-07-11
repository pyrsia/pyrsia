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

use crate::artifact_service::service::ArtifactService;

use super::handlers::blobs::*;
use super::handlers::manifests::*;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::Filter;

pub fn make_docker_routes(
    artifact_service: Arc<Mutex<ArtifactService>>,
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

    let artifact_service_filter = warp::any().map(move || artifact_service.clone());

    let v2_manifests = warp::path!("v2" / "library" / String / "manifests" / String)
        .and(warp::get().or(warp::head()).unify())
        .and(artifact_service_filter.clone())
        .and_then(fetch_manifest);

    let v2_blobs = warp::path!("v2" / "library" / String / "blobs" / String)
        .and(warp::get().or(warp::head()).unify())
        .and(warp::path::end())
        .and(artifact_service_filter)
        .and_then(handle_get_blobs);

    warp::any().and(v2_base.or(v2_manifests).or(v2_blobs))
}
