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
/// Describes a package
pub struct Package {
    /// The id of the namespace that this package is part of.
    namespace_id: String,
    /// The name of this package. Must be unique within a namespace.
    name: String,
    /// The type of package (Docker, Conan, npm, ...)
    pkg_type: String,
    /// ISO-8601 creation time
    creation_time: Option<String>,
    /// ISO-8601 modification time
    modified_time: Option<String>,
    /// Updates to a package should be signed by an identity associated with one of the public keys in the administrators field.
    administrators: Vec<Vec<u8>>,
    /// textual description
    description: Option<String>,
    /// Attributes of a package that don't fit into one of this struct's fields can go in here as JSON
    metadata: Map<String, Value>,
    /// A URL associated with the project.
    project_url: Option<String>,
    /// Known versions of this package.  There should be a PackageVersion describing each one of these.
    versions: Vec<String>,
}
