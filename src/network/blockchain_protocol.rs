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
use libp2p::core::upgrade::{
    read_length_prefixed, read_varint, write_length_prefixed, write_varint, ProtocolName,
};
use libp2p::request_response::RequestResponseCodec;
use log::debug;
use std::io;

#[derive(Debug, Clone)]
pub struct BlockUpdateExchangeProtocol();

#[derive(Clone)]
pub struct BlockUpdateExchangeCodec();
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockUpdateRequest(pub u64, pub Vec<u8>);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockUpdateResponse();

impl ProtocolName for BlockUpdateExchangeProtocol {
    fn protocol_name(&self) -> &[u8] {
        "/pyrsia-blockchain-update-exchange/1".as_bytes()
    }
}
#[async_trait]
impl RequestResponseCodec for BlockUpdateExchangeCodec {
    type Protocol = BlockUpdateExchangeProtocol;
    type Request = BlockUpdateRequest;
    type Response = BlockUpdateResponse;

    //read blockchain request from a peer.
    async fn read_request<T>(
        &mut self,
        _: &BlockUpdateExchangeProtocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        debug!("prysia::blobkchain_protocol::BlockUpdate::read_request received from peer.");
        let mut buff: [u8; 8] = [0; 8];
        let mut size = read_varint(io).await?;
        if size != 8 {
            return Err(io::ErrorKind::InvalidData.into());
        }

        size = io.read(&mut buff).await?;
        if size != 8 {
            return Err(io::ErrorKind::InvalidData.into());
        }

        let block_vec = read_length_prefixed(io, 1_000_000).await?;
        if block_vec.is_empty() {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }

        debug!("Read Blockchain Request: block is {:?}", block_vec);

        Ok(BlockUpdateRequest(u64::from_be_bytes(buff), block_vec))
    }

    //reads blockchain response from the peer
    async fn read_response<T>(
        &mut self,
        _: &BlockUpdateExchangeProtocol,
        _io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        Ok(BlockUpdateResponse())
    }

    //this method send blockchain request from the peer
    async fn write_request<T>(
        &mut self,
        _: &BlockUpdateExchangeProtocol,
        io: &mut T,
        BlockUpdateRequest(block_ordinal, block): BlockUpdateRequest,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        debug!("Write BlockUpdateRequest: {:?}={:?}", block_ordinal, block);
        let data = block_ordinal.to_be_bytes();
        write_varint(io, data.as_ref().len()).await?;
        io.write_all(data.as_ref()).await?;
        io.flush().await?;

        write_length_prefixed(io, block).await?;
        io.close().await?;

        Ok(())
    }

    //this object writes the quality metric to the peer.
    async fn write_response<T>(
        &mut self,
        _: &BlockUpdateExchangeProtocol,
        _io: &mut T,
        BlockUpdateResponse(): BlockUpdateResponse,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        Ok(())
    }
}
