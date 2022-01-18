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

extern crate serde;
extern crate serde_json;
extern crate signed;
extern crate signed_struct;

use crate::node_manager::model::package_type::PackageTypeName;
use signed::signed::Signed;
use signed_struct::signed_struct;

#[signed_struct]
#[derive(Debug,PartialEq)]
/// Describes a name space.
pub struct Namespace {
    /// A uuid that uniquely identifies the name space
    id: String,
    /// The type of package the name space is for (Docker, Conan, ...)
    package_type: PackageTypeName,
    /// Many name spaces are hierarchical. Different package types punctuate the path of a name space. Instead of using punctuation, we put the elements of the path in a Vec.
    namespace_path: String,
    /// Updates to a name space should be signed by an identity associated with one of the public keys in the administrators field.
    #[builder(default = "Vec::new()")]
    administrators: Vec<Vec<u8>>,
    /// ISO-8601 creation time
    creation_time: Option<String>,
    /// ISO-8601 modification time
    modified_time: Option<String>,
}
