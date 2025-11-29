// Bitcoin Primitives and Types
use bitcoin::{block::Checked, consensus::encode, Network, TestnetVersion};

// Bitcoin kernel api
use bitcoinkernel::{core::BlockHashExt, BlockTreeEntry, ChainType};

// Get the user home directory 
use home::home_dir;

// Std library includes
use std::{fs, path::PathBuf};

pub trait ChainExt {
    fn chain_type(&self) -> ChainType;
}

impl ChainExt for Network {
    fn chain_type(&self) -> ChainType {
        match self {
            Network::Bitcoin => ChainType::Mainnet,
            Network::Signet => ChainType::Signet,
            Network::Testnet(TestnetVersion::V3) => ChainType::Testnet,
            Network::Testnet(TestnetVersion::V4) => ChainType::Testnet4,
            Network::Regtest => ChainType::Regtest,
            _ => unimplemented!("unsupported network"),
        }
    }
}

pub trait DirnameExt {
    fn data_dir(&self) -> String;
}

impl DirnameExt for String {
    fn data_dir(&self) -> String {
        let path = match self.strip_prefix("~/") {
            Some(rest) => match home_dir() {
                Some(mut home) => {
                    home.push(rest);
                    home
                }
                None => PathBuf::from(rest),
            },
            None => PathBuf::from(self),
        };
        // Create directories if they don't exist
        fs::create_dir_all(&path).unwrap();

        // Get canonical (full) path
        path.canonicalize().unwrap().to_str().unwrap().to_string()
    }
}

pub fn bitcoin_block_to_kernel_block(block: &bitcoin::Block<Checked>) -> bitcoinkernel::Block {
    let ser_block = encode::serialize(block);
    bitcoinkernel::Block::try_from(ser_block.as_slice()).unwrap()
}

pub fn get_block_hash(index: BlockTreeEntry) -> bitcoin::BlockHash {
    bitcoin::BlockHash::from_byte_array(index.block_hash().to_bytes())
}
