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
use std::path::PathBuf;
use thiserror::Error;

use crate::structures::header::Ordinal;

#[derive(Debug, Error)]
pub enum BlockchainError {
    #[error("Anyhow Error")]
    AnyhowError(#[from] anyhow::Error),
    #[error("Serialization Error")]
    SerializationError(#[from] bincode::Error),
    #[error("IO Error")]
    IOError(#[from] io::Error),
    #[error("Local Blockchain does not exist!")]
    EmptyBlockchain,
    #[error("Invalid Blockchain Argument")]
    InvalidBlockchainArgument,
    #[error("Invalid Blockchain Command")]
    InvalidBlockchainCmd,
    #[error("Invalid Blockchain Length: {0}")]
    InvalidBlockchainLength(usize),
    #[error("Blockchain Start postion: {0} is greater than End postion: {1} ")]
    InvalidBlockchainPosition(usize, usize),
    #[error("Invalid Blockchain Ordinal: {0}")]
    InvalidBlockchainOrdinal(Ordinal),
    #[error("Blockchain: Key {0} is not valid Ed25519 format")]
    InvalidKey(String),
    #[error("Lagging Blockchain Data")]
    LaggingBlockchainData,
    #[error("Invalid storage path: {0}")]
    InvalidStoragePath(PathBuf),
}
