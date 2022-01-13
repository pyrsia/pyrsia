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

extern crate base64;
extern crate reqwest;
extern crate serde;
extern crate serde_json;
extern crate synapse_rpc as rpc;
extern crate tungstenite as ws;
extern crate url;

use error_chain::bail;
use std::process;

use super::error::{ErrorKind, Result, ResultExt};
use rpc::criterion::{Criterion, Operation, Value};
use rpc::resource::{CResourceUpdate, Resource, ResourceKind, SResourceUpdate, Server};
use synapse_rpc::message::{self, CMessage, SMessage};
use log::{info, error};

use super::client::Client;
use url::Url;

pub async fn add_torrent(
    server: &str,
    pass: &str,
    directory: Option<&str>,
    files: Vec<&str>,
) -> Result<()> {
    let mut url = match Url::parse(server) {
        Ok(url) => url,
        Err(e) => {
            error!("Server URL {} is not valid: {}", server, e);
            process::exit(1);
        }
    };
    url.query_pairs_mut().append_pair("password", pass);

    let client = match Client::new(url.clone()) {
        Ok(c) => c,
        Err(_) => {
            error!("Failed to connect to synapse, ensure your URI and password are correct");
            process::exit(1);
        }
    };
    add(
        client,
        url.as_str(),
        files,
        directory,
        true,  // paused
        false, // imported
    )
}

fn add(
    mut c: Client,
    _url: &str,
    files: Vec<&str>,
    dir: Option<&str>,
    start: bool,
    _import: bool,
) -> Result<()> {
    for file in files {
        if let Ok(magnet) = Url::parse(file) {
            add_magnet(&mut c, magnet, dir, start)?;
        }
    }
    Ok(())
}

fn add_magnet(c: &mut Client, magnet: Url, dir: Option<&str>, start: bool) -> Result<()> {
    let msg = CMessage::UploadMagnet {
        serial: c.next_serial(),
        uri: magnet.as_str().to_owned(),
        path: dir.as_ref().map(|d| d.to_string()),
        start,
    };
    match c.rr(msg)? {
        SMessage::ResourcesExtant { ids, .. } => {
            get_(c, ids[0].as_ref(), "text")?;
        }
        SMessage::InvalidRequest(message::Error { reason, .. }) => {
            bail!("{}", reason);
        }
        _ => {
            bail!("Failed to receieve upload acknowledgement from synapse");
        }
    }
    Ok(())
}

pub fn get_(c: &mut Client, id: &str, output: &str) -> Result<()> {
    let res = get_resources(c, vec![id.to_owned()])?;
    if res.is_empty() {
        bail!("Resource not found");
    }
    match output {
        "text" => {
            info!("{}", res[0]);
        }
        "json" => {
            info!(
                "{}",
                serde_json::to_string_pretty(&res[0]).chain_err(|| ErrorKind::Serialization)?
            );
        }
        _ => unreachable!(),
    }
    Ok(())
}

fn get_resources(c: &mut Client, ids: Vec<String>) -> Result<Vec<Resource>> {
    let msg = CMessage::Subscribe {
        serial: c.next_serial(),
        ids: ids.clone(),
    };
    let unsub = CMessage::Unsubscribe {
        serial: c.next_serial(),
        ids,
    };

    let resources = if let SMessage::UpdateResources { resources, .. } = c.rr(msg)? {
        resources
    } else {
        bail!("Failed to received torrent resource list!");
    };

    c.send(unsub)?;

    let mut results = Vec::new();
    for r in resources {
        if let SResourceUpdate::Resource(res) = r {
            results.push(res.into_owned());
        } else {
            bail!("Failed to received full resource!");
        }
    }
    Ok(results)
}
