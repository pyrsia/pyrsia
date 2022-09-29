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

use crate::artifact_service::model::PackageType;
use async_trait::async_trait;
use futures::prelude::*;
use libp2p::core::upgrade::{read_length_prefixed, write_length_prefixed, ProtocolName};
use libp2p::request_response::RequestResponseCodec;
use log::debug;
use std::io;
use std::str::FromStr;

#[derive(Debug, Clone)]
pub struct BuildExchangeProtocol();
/// The `BuildExchangeCodec` defines the request and response types
/// for the [`RequestResponse`](crate::RequestResponse) protocol for
/// exchanging builds. At the moment, the implementation for
/// encoding/decoding writes all bytes of a single artifact at once.
#[derive(Clone)]
pub struct BuildExchangeCodec();
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildRequest(pub PackageType, pub String);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildResponse();

impl ProtocolName for BuildExchangeProtocol {
    fn protocol_name(&self) -> &[u8] {
        "/build-exchange/1".as_bytes()
    }
}

#[async_trait]
impl RequestResponseCodec for BuildExchangeCodec {
    type Protocol = BuildExchangeProtocol;
    type Request = BuildRequest;
    type Response = BuildResponse;

    async fn read_request<T>(
        &mut self,
        _: &BuildExchangeProtocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        debug!("Reading BuildRequest...");

        let hash_vec0 = read_length_prefixed(io, 1_000_000).await?;
        if hash_vec0.is_empty() {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }

        let package_type_string = String::from_utf8(hash_vec0).unwrap();
        let package_type = PackageType::from_str(&package_type_string).unwrap();

        let hash_vec1 = read_length_prefixed(io, 1_000_000).await?;
        if hash_vec1.is_empty() {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }

        let package_specific_id = String::from_utf8(hash_vec1).unwrap();
        debug!(
            "Read BuildRequest: {:?}:{}",
            package_type, package_specific_id
        );

        Ok(BuildRequest(package_type, package_specific_id))
    }

    async fn read_response<T>(
        &mut self,
        _: &BuildExchangeProtocol,
        _io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        Ok(BuildResponse())
    }

    async fn write_request<T>(
        &mut self,
        _: &BuildExchangeProtocol,
        io: &mut T,
        BuildRequest(package_type, package_specific_id): BuildRequest,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        debug!(
            "Write BuildRequest: {:?}: {}",
            package_type, package_specific_id
        );

        write_length_prefixed(io, package_type.to_string()).await?;
        write_length_prefixed(io, package_specific_id).await?;
        io.close().await?;

        Ok(())
    }

    async fn write_response<T>(
        &mut self,
        _: &BuildExchangeProtocol,
        _io: &mut T,
        BuildResponse(): BuildResponse,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        Ok(())
    }
}
