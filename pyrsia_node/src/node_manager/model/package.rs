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
extern crate pyrsia_client_lib;
extern crate serde;
extern crate serde_json;

use pyrsia_client_lib::signed::Signed;
use serde_json::{Map, Value};
use signed_struct::signed_struct;

#[signed_struct]
#[derive(Debug)]
pub struct Package {
    name: String,
    pkg_type: String,
    namespace_id: String,
    creation_time: Option<String>,
    modified_time: Option<String>,
    administrator: Option<Vec<u8>>,
    description: Option<String>,
    metadata: Map<String, Value>,
    project_url: Option<String>,
    project_name: Option<String>,
    versions: Vec<u8>,
}
