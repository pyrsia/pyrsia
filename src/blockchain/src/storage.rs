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
use super::*;
use libp2p::identity;

pub const BLOCK_FILE_PATH: &str = "./blockchain_storage";

//Add genesis block to the file
pub fn append_genesis_block(path: &str, key: &identity::ed25519::Keypair) {
    use blockchain::GenesisBlock;
    use std::fs::OpenOptions;
    use std::io::Write;

    let g_block = GenesisBlock::new(key);
    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(path)
        .expect("cannot open file");

    file.write_all(serde_json::to_string(&g_block).unwrap().as_bytes())
        .expect("write failed");
    file.write_all(b"\n").expect("write failed");
}

//read a last block from the file, and return this block hash, this block number and this block committer
pub fn read_last_block(path: &str) -> (header::HashDigest, u128, header::Address) {
    use std::io::{BufRead, BufReader};
    let file = std::fs::File::open(path).unwrap();

    let buffered = BufReader::new(file);
    let line = buffered.lines().last().expect("stdin to read").unwrap();

    let block: block::Block = match serde_json::from_str(&line) {
        Ok(v) => v,
        Err(_) => return parse_genesis_block(&line),
    };

    (
        block.header.current_hash,
        block.header.number,
        block.header.committer,
    )
}

// Unformat the genesis block json string
pub fn parse_genesis_block(line: &str) -> (header::HashDigest, u128, header::Address) {
    let genesis_block: blockchain::GenesisBlock = serde_json::from_str(line).unwrap();
    (
        genesis_block.header.current_hash,
        genesis_block.header.number,
        genesis_block.header.committer,
    )
}
//Write a block to the file
pub fn write_block(path: &str, block: block::Block) {
    use std::fs::OpenOptions;
    use std::io::Write;

    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(path)
        .expect("cannot open file");

    file.write_all(serde_json::to_string(&block).unwrap().as_bytes())
        .expect("write failed");
    file.write_all(b"\n").expect("write failed");
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::identity;

    #[test]
    fn test_write_read() -> Result<(), String> {
        let keypair = identity::ed25519::Keypair::generate();
        let local_id = header::hash(&block::get_publickey_from_keypair(&keypair).encode());
        let mut transactions = vec![];
        let data = "Hello First Transaction";
        let transaction = block::Transaction::new(
            block::PartialTransaction::new(
                block::TransactionType::Create,
                local_id,
                data.as_bytes().to_vec(),
            ),
            &keypair,
        );
        transactions.push(transaction);
        let block_header = header::Header::new(header::PartialHeader::new(
            header::hash(b""),
            local_id,
            header::hash(b""),
            1,
        ));

        let block = block::Block::new(block_header, transactions.to_vec(), &keypair);
        append_genesis_block(BLOCK_FILE_PATH, &keypair);
        write_block(BLOCK_FILE_PATH, block);
        let (_, number, _) = read_last_block(BLOCK_FILE_PATH);

        assert_eq!(1, number);
        Ok(())
    }
}
