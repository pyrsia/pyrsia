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

use async_trait::async_trait;
use futures::prelude::*;
use libp2p::core::upgrade::{read_length_prefixed, write_length_prefixed, ProtocolName};
use libp2p::request_response::RequestResponseCodec;
use log::debug;
use std::io;

use crate::blockchain_service::service::{MAX_BLOCK_NUMBER_PER_MESSAGE, MAX_BLOCK_SIZE};

#[derive(Debug, Clone)]
pub struct BlockchainExchangeProtocol();

#[derive(Clone)]
pub struct BlockchainExchangeCodec();
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockchainRequest(pub Vec<u8>);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockchainResponse(pub Vec<u8>);

impl ProtocolName for BlockchainExchangeProtocol {
    fn protocol_name(&self) -> &[u8] {
        "/pyrsia-blockchain-update-exchange/1".as_bytes()
    }
}
#[async_trait]
impl RequestResponseCodec for BlockchainExchangeCodec {
    type Protocol = BlockchainExchangeProtocol;
    type Request = BlockchainRequest;
    type Response = BlockchainResponse;

    ///This method reads the blockchain request from the peer.
    async fn read_request<T>(
        &mut self,
        _: &BlockchainExchangeProtocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let buffer =
            read_length_prefixed(io, (MAX_BLOCK_NUMBER_PER_MESSAGE + 1) * MAX_BLOCK_SIZE).await?;
        if buffer.is_empty() {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }

        debug!("Blockchain::read_request starts");

        Ok(BlockchainRequest(buffer))
    }

    ///This method reads the blockchain response from the peer
    async fn read_response<T>(
        &mut self,
        _: &BlockchainExchangeProtocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let buffer =
            read_length_prefixed(io, (MAX_BLOCK_NUMBER_PER_MESSAGE + 1) * MAX_BLOCK_SIZE).await?;
        if buffer.is_empty() {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }

        debug!("Blockchain::read_response starts");

        Ok(BlockchainResponse(buffer))
    }

    ///This method sends a blockchain request to the peer
    async fn write_request<T>(
        &mut self,
        _: &BlockchainExchangeProtocol,
        io: &mut T,
        BlockchainRequest(data): BlockchainRequest,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        debug!("Blockchain::write_request starts");

        write_length_prefixed(io, data).await?;
        io.close().await?;

        Ok(())
    }

    ///This method sends a blockchain request to the peer
    async fn write_response<T>(
        &mut self,
        _: &BlockchainExchangeProtocol,
        io: &mut T,
        BlockchainResponse(data): BlockchainResponse,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        debug!("Blockchain::write_response starts");

        write_length_prefixed(io, data).await?;
        io.close().await?;

        Ok(())
    }
}
