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

use super::{network::NetworkData, structures::block::Block, structures::header::Ordinal};
use async_trait::async_trait;
use futures::channel::{
    mpsc,
    mpsc::{UnboundedReceiver, UnboundedSender},
};
use log::{debug, error};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};

pub struct DataStore {
    next_message_id: u32,
    current_block: Arc<Mutex<Block>>,
    available_blocks: HashSet<Ordinal>,
    message_requirements: HashMap<u32, usize>,
    dependent_messages: HashMap<Block, Vec<u32>>,
    pending_messages: HashMap<u32, NetworkData>,
    messages_for_member: UnboundedSender<NetworkData>,
}

impl DataStore {
    pub fn new(
        current_block: Arc<Mutex<Block>>,
        messages_for_member: UnboundedSender<NetworkData>,
    ) -> Self {
        let available_blocks = (0..=current_block.lock().unwrap().header.ordinal).collect();
        DataStore {
            next_message_id: 0,
            current_block,
            available_blocks,
            message_requirements: HashMap::new(),
            dependent_messages: HashMap::new(),
            pending_messages: HashMap::new(),
            messages_for_member,
        }
    }

    fn add_pending_message(&mut self, message: NetworkData, requirements: Vec<Block>) {
        let message_id = self.next_message_id;
        // Whatever test you are running should end before this becomes a problem.
        self.next_message_id += 1;
        for block in requirements.iter() {
            self.dependent_messages
                .entry(block.clone())
                .or_insert_with(Vec::new)
                .push(message_id);
        }
        self.message_requirements
            .insert(message_id, requirements.len());
        self.pending_messages.insert(message_id, message);
    }

    pub fn add_message(&mut self, message: NetworkData) {
        let requirements: Vec<_> = message
            .included_data()
            .into_iter()
            .filter(|b| !self.available_blocks.contains(&b.header.ordinal))
            .collect();
        if requirements.is_empty() {
            self.messages_for_member
                .unbounded_send(message)
                .expect("member accept messages");
        } else {
            self.add_pending_message(message, requirements.into_iter().collect());
        }
    }

    fn push_messages(&mut self, block: Block) {
        for message_id in self
            .dependent_messages
            .entry(block.clone())
            .or_insert_with(Vec::new)
            .iter()
        {
            *self
                .message_requirements
                .get_mut(message_id)
                .expect("there are some requirements") -= 1;
            if self.message_requirements[message_id] == 0 {
                let message = self
                    .pending_messages
                    .remove(message_id)
                    .expect("there is a pending message");
                self.messages_for_member
                    .unbounded_send(message)
                    .expect("member accept messages");
                self.message_requirements.remove(message_id);
            }
        }
        self.dependent_messages.remove(&block);
    }

    pub fn add_block(&mut self, block: Block) {
        debug!("Added block {:?}.", block);
        self.available_blocks.insert(block.header.ordinal);
        self.push_messages(block);
        while self
            .available_blocks
            .contains(&(self.current_block.lock().unwrap().header.ordinal + 1))
        {
            self.current_block.lock().unwrap().header.ordinal += 1;
        }
        debug!("Updated chain {:?}", self.available_blocks);
    }
}

#[derive(Clone)]
pub struct DataProvider {
    current_block: Arc<Mutex<Block>>,
}

#[async_trait]
impl aleph_bft::DataProvider<Block> for DataProvider {
    async fn get_data(&mut self) -> Block {
        self.current_block.lock().unwrap().clone()
    }
}

impl DataProvider {
    // TODO(prince-chrismc): Initial Block?
    pub fn new(initial_block: Arc<Mutex<Block>>) -> Self {
        DataProvider {
            current_block: initial_block,
        }
    }
}

pub struct FinalizationProvider {
    tx: UnboundedSender<Block>,
}

#[async_trait]
impl aleph_bft::FinalizationHandler<Block> for FinalizationProvider {
    async fn data_finalized(&mut self, d: Block) {
        if let Err(e) = self.tx.unbounded_send(d) {
            error!("Error when sending data from FinalizationProvider {:?}.", e);
        }
    }
}

impl FinalizationProvider {
    pub fn new() -> (Self, UnboundedReceiver<Block>) {
        let (tx, rx) = mpsc::unbounded();

        (Self { tx }, rx)
    }
}
