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

use crate::build_service::model::BuildStatus;
use async_trait::async_trait;
use futures::prelude::*;
use libp2p::core::upgrade::{read_length_prefixed, write_length_prefixed, ProtocolName};
use libp2p::request_response::RequestResponseCodec;
use log::debug;
use std::io;

#[derive(Debug, Clone)]
pub struct BuildStatusExchangeProtocol();
#[derive(Clone)]
pub struct BuildStatusExchangeCodec();
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildStatusRequest(pub String);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildStatusResponse(pub BuildStatus);

impl ProtocolName for BuildStatusExchangeProtocol {
    fn protocol_name(&self) -> &[u8] {
        "/build-status-exchange/1".as_bytes()
    }
}

#[async_trait]
impl RequestResponseCodec for BuildStatusExchangeCodec {
    type Protocol = BuildStatusExchangeProtocol;
    type Request = BuildStatusRequest;
    type Response = BuildStatusResponse;

    async fn read_request<T>(
        &mut self,
        _: &BuildStatusExchangeProtocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        debug!("Reading BuildStatusRequest...");

        let hash_vec0 = read_length_prefixed(io, 1_000_000).await?;
        if hash_vec0.is_empty() {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }

        let build_id = String::from_utf8(hash_vec0).unwrap();

        Ok(BuildStatusRequest(build_id))
    }

    async fn read_response<T>(
        &mut self,
        _: &BuildStatusExchangeProtocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        debug!("Reading BuildStatusResponse...");
        let hash_vec = read_length_prefixed(io, 1_000_000).await?;
        if hash_vec.is_empty() {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }

        let status: BuildStatus = bincode::deserialize(&hash_vec).unwrap();

        Ok(BuildStatusResponse(status))
    }

    async fn write_request<T>(
        &mut self,
        _: &BuildStatusExchangeProtocol,
        io: &mut T,
        BuildStatusRequest(build_id): BuildStatusRequest,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        debug!("Write BuildStatusRequest: build_id: {:?}", build_id);

        write_length_prefixed(io, build_id.to_string()).await?;
        io.close().await?;

        Ok(())
    }

    async fn write_response<T>(
        &mut self,
        _: &BuildStatusExchangeProtocol,
        io: &mut T,
        BuildStatusResponse(status): BuildStatusResponse,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        debug!("Write BuildStatusResponse: {:?}", status);

        let data = bincode::serialize(&status).unwrap();
        write_length_prefixed(io, data).await?;

        Ok(())
    }
}
