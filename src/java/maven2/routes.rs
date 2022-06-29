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

use super::handlers::maven_artifacts::handle_get_maven_artifact;
use crate::artifact_service::service::ArtifactService;
use log::debug;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::Filter;

pub fn make_maven_routes(
    artifact_service: Arc<Mutex<ArtifactService>>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let maven2_root = warp::path("maven2")
        .and(warp::path::full())
        .map(|path: warp::path::FullPath| {
            let full_path: String = path.as_str().to_string();
            debug!("route full path: {}", full_path);
            full_path
        })
        .and_then(move |full_path| handle_get_maven_artifact(artifact_service.clone(), full_path));

    warp::any().and(maven2_root)
}
