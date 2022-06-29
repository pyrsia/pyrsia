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

use super::handlers::swarm::*;
use crate::artifact_service::service::ArtifactService;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::Filter;

pub fn make_node_routes(
    artifact_service: Arc<Mutex<ArtifactService>>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let artifact_service_peers = artifact_service.clone();

    let peers = warp::path!("peers")
        .and(warp::get())
        .and(warp::path::end())
        .and_then(move || handle_get_peers(artifact_service_peers.clone()));

    let status = warp::path!("status")
        .and(warp::get())
        .and(warp::path::end())
        .and_then(move || handle_get_status(artifact_service.clone()));

    warp::any().and(peers.or(status))
}
