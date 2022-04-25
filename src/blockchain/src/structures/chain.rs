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

use codec::{Decode, Encode};
use serde::{Deserialize, Serialize};

use super::block::Block;

#[derive(Serialize, Deserialize, Debug, Default, Clone, Decode, Encode, Hash, PartialEq, Eq)]
pub struct Chain {
    // TODO(prince-chrismc): This eventually needs to be an ordered set so block sequence is always sorted by ordinal
    pub blocks: Vec<Block>,
}
