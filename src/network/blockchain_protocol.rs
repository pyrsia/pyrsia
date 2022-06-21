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
use pyrsia_blockchain_network::structures::transaction::TransactionType;
use serde::{Deserialize, Serialize};
use std::io;

#[derive(Debug, Clone)]
pub struct BlockchainExchangeProtocol();

#[derive(Clone)]
pub struct BlockchainExchangeCodec();
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockchainRequest(pub TransactionType, pub Vec<u8>);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlockchainResponse(pub Option<u64>);

impl ProtocolName for BlockchainExchangeProtocol {
    fn protocol_name(&self) -> &[u8] {
        "/pyrsia-blockchain-exchange/1".as_bytes()
    }
}
#[async_trait]
impl RequestResponseCodec for BlockchainExchangeCodec {
    type Protocol = BlockchainExchangeProtocol;
    type Request = BlockchainRequest;
    type Response = BlockchainResponse;

    //read blockchain request from a peer.
    async fn read_request<T>(
        &mut self,
        _: &BlockchainExchangeProtocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        debug!("prysia::blobkchain_protocol::read_request received from peer.");

        let type_vec = read_length_prefixed(io, 1_000).await?;
        if type_vec.is_empty() {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }

        let payload_vec = read_length_prefixed(io, 1_000_000).await?;
        if payload_vec.is_empty() {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }

        let transaction_operation = match type_vec[0] {
            1 => TransactionType::AddAuthority,
            2 => TransactionType::AddTransparencyLog,
            _ => return Err(io::ErrorKind::InvalidData.into()),
        };

        debug!(
            "Read Blockchain Request: {:?}={:?}",
            transaction_operation, payload_vec
        );

        Ok(BlockchainRequest(transaction_operation, payload_vec))
    }

    //reads blockchain response from the peer
    async fn read_response<T>(
        &mut self,
        _: &BlockchainExchangeProtocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut buff: [u8; 8] = [0; 8];
        let mut blockchain_ordinal = None;
        let mut size = read_varint(io).await?;
        if size != 8 {
            blockchain_ordinal = None;
        }

        size = io.read(&mut buff).await?;
        if size != 8 {
            blockchain_ordinal = None;
        }

        blockchain_ordinal = Some(u64::from_be_bytes(buff));
        debug!(
            "prysia::blobkchain_protocol::read_response Reading response to blockchain request with value ={:?}",
            blockchain_ordinal
        );
        Ok(BlockchainResponse(blockchain_ordinal))
    }

    //this method send blockchain request from the peer
    async fn write_request<T>(
        &mut self,
        _: &BlockchainExchangeProtocol,
        io: &mut T,
        BlockchainRequest(transaction_operation, payload): BlockchainRequest,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        debug!(
            "Write BlockchainRequest: {:?}={:?}",
            transaction_operation, payload
        );

        let transaction_data_operator: Vec<u8> = match transaction_operation {
            TransactionType::AddAuthority => vec![1],
            TransactionType::AddTransparencyLog => vec![2],
        };

        write_length_prefixed(io, transaction_data_operator).await?;
        write_length_prefixed(io, payload).await?;
        io.close().await?;

        Ok(())
    }

    //this object writes the quality metric to the peer.
    async fn write_response<T>(
        &mut self,
        _: &BlockchainExchangeProtocol,
        io: &mut T,
        BlockchainResponse(data): BlockchainResponse,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        debug!(
            "prysia::blobkchain_protocol::write_response Send blockchain response with value = {:?}",
            data
        );

        match data {
            Some(x) => {
                let data = u64::to_be_bytes(x);
                write_varint(io, data.as_ref().len()).await?;
                io.write_all(data.as_ref()).await?;
                io.flush().await?;
            }
            None => {
                write_varint(io, 0).await?;
                io.flush().await?;
            }
        }

        io.close().await?;

        Ok(())
    }
}
