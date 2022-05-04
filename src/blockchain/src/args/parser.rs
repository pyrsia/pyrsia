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

use clap::Parser;

pub const DEFAULT_BLOCK_FILE_PATH: &str = "./blockchain_storage";
pub const DEFAULT_BLOCK_KEYPAIR_FILENAME: &str = ".block_keypair";

/// Application to connect to and participate in the Pyrsia blockchain network
#[derive(Debug, Parser)]
#[clap(long_about = None)]
pub struct BlockchainNodeArgs {
    /// A string to specify the keypair filename
    #[clap(long, short = 'K', default_value = DEFAULT_BLOCK_KEYPAIR_FILENAME)]
    pub key_filename: String,
}
