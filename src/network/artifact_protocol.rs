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
use std::io;

#[derive(Debug, Clone)]
pub struct ArtifactExchangeProtocol();
/// The `ArtifactExchangeCodec` defines the request and response types
/// for the [`RequestResponse`](crate::RequestResponse) protocol for
/// exchanging artifacts. At the moment, the implementation for
/// encoding/decoding writes all bytes of a single artifact at once.
#[derive(Clone)]
pub struct ArtifactExchangeCodec();
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactRequest(pub String);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactResponse(pub Vec<u8>);

impl ProtocolName for ArtifactExchangeProtocol {
    fn protocol_name(&self) -> &[u8] {
        "/artifact-exchange/1".as_bytes()
    }
}

#[async_trait]
impl RequestResponseCodec for ArtifactExchangeCodec {
    type Protocol = ArtifactExchangeProtocol;
    type Request = ArtifactRequest;
    type Response = ArtifactResponse;

    async fn read_request<T>(
        &mut self,
        _: &ArtifactExchangeProtocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let vec = read_length_prefixed(io, 100_000_000).await?;

        if vec.is_empty() {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }

        Ok(ArtifactRequest(String::from_utf8(vec).unwrap()))
    }

    async fn read_response<T>(
        &mut self,
        _: &ArtifactExchangeProtocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let vec = read_length_prefixed(io, 100_000_000).await?;

        if vec.is_empty() {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }

        Ok(ArtifactResponse(vec))
    }

    async fn write_request<T>(
        &mut self,
        _: &ArtifactExchangeProtocol,
        io: &mut T,
        ArtifactRequest(data): ArtifactRequest,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        write_length_prefixed(io, data).await?;
        io.close().await?;

        Ok(())
    }

    async fn write_response<T>(
        &mut self,
        _: &ArtifactExchangeProtocol,
        io: &mut T,
        ArtifactResponse(data): ArtifactResponse,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        write_length_prefixed(io, data).await?;
        io.close().await?;

        Ok(())
    }
}
