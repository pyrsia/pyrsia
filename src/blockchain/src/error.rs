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

use std::io;
use thiserror::Error;

use crate::structures::header::Ordinal;

#[derive(Debug, Error)]
pub enum BlockchainError {
    #[error("Anyhow Error")]
    AnyhowError(#[from] anyhow::Error),
    #[error("IO Error")]
    IOError(#[from] io::Error),
    #[error("Invalid blockchain command")]
    InvalidBlockchainCmd,
    #[error("Invalid blockchain length: {0}")]
    InvalidBlockchainLength(usize),
    #[error("Blockchain start postion: {0} is greater than end postion: {1} ")]
    InvalidBlockchainPosition(usize, usize),
    #[error("Invalid blockchain Ordinal: {0}")]
    InvalidBlockchainOrdinal(Ordinal),
}
