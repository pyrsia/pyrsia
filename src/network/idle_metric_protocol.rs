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
use libp2p::core::upgrade::{read_varint, write_length_prefixed, ProtocolName};
use libp2p::request_response::RequestResponseCodec;
use log::debug;
use serde::{Deserialize, Serialize};
use std::io;

/// The `IdleMetricExchangeCodec` defines the request and response types
/// for the [`RequestResponse`](crate::RequestResponse) protocol for
/// exchanging peer metrics. The peer metric is passed through the framework
/// in the PeerMetrics structure but over the network as the bytes needed
/// for passing a floating point as bits of the idle metric field of the
/// PeerMetrics structure.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PeerMetrics {
    pub idle_metric: [u8; 8],
}

impl PartialEq for PeerMetrics {
    fn eq(&self, other: &Self) -> bool {
        self.idle_metric == other.idle_metric
    }
}

impl Eq for PeerMetrics {}

impl AsRef<[u8]> for PeerMetrics {
    fn as_ref(&self) -> &[u8] {
        &self.idle_metric
    }
}

#[derive(Debug, Clone)]
pub struct IdleMetricExchangeProtocol();

#[derive(Clone)]
pub struct IdleMetricExchangeCodec();
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdleMetricRequest();
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdleMetricResponse(pub PeerMetrics);

impl ProtocolName for IdleMetricExchangeProtocol {
    fn protocol_name(&self) -> &[u8] {
        "/metric-exchange/1".as_bytes()
    }
}
#[async_trait]
impl RequestResponseCodec for IdleMetricExchangeCodec {
    type Protocol = IdleMetricExchangeProtocol;
    type Request = IdleMetricRequest;
    type Response = IdleMetricResponse;

    //request for idle metric from a peer.
    async fn read_request<T>(
        &mut self,
        _: &IdleMetricExchangeProtocol,
        _io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        debug!("p2p::idle_metric_protocol::read_request received from peer.",);
        //unlike the artifact, there is no has parameter there is only one metric possible
        Ok(IdleMetricRequest()) //TODO: can I just return OK from here with no type
    }

    //reads the peer metric from the peer
    async fn read_response<T>(
        &mut self,
        _: &IdleMetricExchangeProtocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let mut buff: [u8; 8] = [0; 8];
        let mut size = read_varint(io).await?;
        if size != 8 {
            return Err(io::ErrorKind::InvalidData.into());
        }

        size = io.read(&mut buff).await?;
        if size != 8 {
            return Err(io::ErrorKind::InvalidData.into());
        }

        let metric = PeerMetrics { idle_metric: buff };
        debug!(
            "p2p::idle_metric_protocol::read_response Reading response to idle metric request with value ={:?}",
            metric.idle_metric
        );
        Ok(IdleMetricResponse(metric))
    }

    //this method request the idle metric from the peer
    async fn write_request<T>(
        &mut self,
        _: &IdleMetricExchangeProtocol,
        io: &mut T,
        IdleMetricRequest(): IdleMetricRequest,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        debug!(
            "p2p::idle_metric_protocol::write_request writing a request to peer for and idle metric",
        );
        io.close().await?;
        Ok(())
    }

    //this object writes the quality metric to the peer.
    async fn write_response<T>(
        &mut self,
        _: &IdleMetricExchangeProtocol,
        io: &mut T,
        IdleMetricResponse(data): IdleMetricResponse,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        debug!(
            "p2p::idle_metric_protocol::write_response sending PeerMetric metric value {:?}",
            data
        );
        write_length_prefixed(io, data.idle_metric).await?;
        io.close().await?;

        Ok(())
    }
}
