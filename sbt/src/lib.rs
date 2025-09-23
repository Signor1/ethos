//!
//! # Soulbound Token (SBT) Contract
//! A non-transferable ERC-721 token. New instances will be deployed by the Factory.
//!
// Allow `cargo stylus export-abi` to generate a main function.
#![cfg_attr(not(any(test, feature = "export-abi")), no_main)]
#![cfg_attr(not(any(test, feature = "export-abi")), no_std)]

mod base64;
mod generator;

#[macro_use]
extern crate alloc;

use alloc::string::String;
use alloc::vec::Vec;
use alloy_primitives::{Address, U256};
use openzeppelin_stylus::token::erc721::{self, Erc721};

use stylus_sdk::{
    alloy_sol_types::sol,
    prelude::*,
    storage::{StorageMap, StorageU256},
};

sol! {
    // Events
    event Minted(address indexed to, uint256 indexed tokenId);
    event BatchMinted(address indexed issuer, uint256 count);

    // Errors
    error NotTransferable();
    error Unauthorized();
    error TokenNotExists();
    error ZeroAddress();
    error EmptyArray();
}

#[derive(SolidityError)]
pub enum SBTErrors {
    NotTransferable(NotTransferable),
    Unauthorized(Unauthorized),
    TokenNotExists(TokenNotExists),
    ZeroAddress(ZeroAddress),
    EmptyArray(EmptyArray),
    Erc721(erc721::Error),
}

impl From<erc721::Error> for SBTErrors {
    fn from(err: erc721::Error) -> Self {
        SBTErrors::Erc721(err)
    }
}

sol_storage! {
    #[entrypoint]
    pub struct SBT {
        #[borrow]
        Erc721 erc721;

        /// The address of the issuer who created this SBT collection.
        address issuer;
        string name;
        string symbol;
        /// The next token ID to be minted.
        uint256 next_token_id;
        /// Mapping from token ID to unique bytes for on-chain uri
        mapping(uint256 => bytes32) entropy;
    }
}

impl SBT {
    fn generate_entropy(&self) -> FixedBytes<32> {
        let block_number = self.vm().block_number();
        let msg_sender = self.vm().msg_sender();
        let chain_id = self.vm().chain_id();

        let hash_data = (block_number, msg_sender, chain_id).abi_encode_sequence();

        keccak(&hash_data)
    }
}

#[public]
#[inherit(Erc721)]
impl SBT {
    #[constructor]
    fn constructor(
        &mut self,
        name: String,
        symbol: String,
        issuer: Address,
    ) -> Result<(), SBTErrors> {
        if issuer.is_zero() {
            return Err(SBTErrors::ZeroAddress(ZeroAddress {}));
        }

        // Set SBT specific storage
        self.name.set_str(&name);
        self.symbol.set_str(&symbol);
        self.issuer.set(issuer);
        self.next_token_id.set(U256::from(1));

        Ok(())
    }

    fn name(&self) -> String {
        self.name.get_string()
    }

    fn symbol(&self) -> String {
        self.symbol.get_string()
    }

    #[selector(name = "tokenURI")]
    fn token_uri(&self, token_id: U256) -> Result<String, SBTErrors> {
        let seed = self.entropy.get(token_id);
        let generator = generator::SBTGenerator::new(seed);
        let metadata = generator.metadata();
        Ok(metadata)
    }

    fn mint_to_one(&mut self, to: Address) -> Result<(), SBTErrors> {
        // Only issuer can mint
        if self.vm().msg_sender() != self.issuer.get() {
            return Err(SBTErrors::Unauthorized(Unauthorized {}));
        }

        if to.is_zero() {
            return Err(SBTErrors::ZeroAddress(ZeroAddress {}));
        }

        let token_id = self.next_token_id.get();

        let seed = self.generate_entropy();
        self.entropy.setter(token_id).set(seed);

        // Mint via ERC721 base contract
        self.erc721._mint(to, token_id)?;

        // Increment next token ID
        self.next_token_id.set(token_id + U256::from(1));

        log(
            self.vm(),
            Minted {
                to,
                tokenId: token_id,
            },
        );

        Ok(token_id)
    }

    fn mint_to_many(&mut self, recipients: Vec<Address>) -> Result<Vec<U256>, SBTErrors> {
        // Only issuer can mint
        if self.vm().msg_sender() != self.issuer.get() {
            return Err(SBTErrors::Unauthorized(Unauthorized {}));
        }

        if recipients.is_empty() {
            return Err(SBTErrors::EmptyArray(EmptyArray {}));
        }

        let mut token_ids = Vec::new();
        let mut current_token_id = self.next_token_id.get();

        for recipient in recipients.iter() {
            if recipient.is_zero() {
                return Err(SBTErrors::ZeroAddress(ZeroAddress {}));
            }

            let seed = self.generate_entropy();
            self.entropy.setter(current_token_id).set(seed);

            // Mint via ERC721 base contract
            self.erc721._mint(*recipient, current_token_id)?;

            log(
                self.vm(),
                Minted {
                    to: *recipient,
                    tokenId: current_token_id,
                },
            );

            token_ids.push(current_token_id);
            current_token_id += U256::from(1);
        }

        // Update next token ID
        self.next_token_id.set(current_token_id);

        log(
            self.vm(),
            BatchMinted {
                issuer: self.vm().msg_sender(),
                count: U256::from(recipients.len()),
            },
        );

        Ok(token_ids)
    }

    fn get_issuer(&self) -> Address {
        self.issuer.get()
    }

    fn get_next_token_id(&self) -> U256 {
        self.next_token_id.get()
    }

    fn total_supply(&self) -> U256 {
        self.next_token_id.get() - U256::from(1)
    }

    ///#######################################################
    /// DISABLED TRANSFER FUNCTIONS (Soulbound implementation)
    ///########################################################

    /// Disabled: SBTs cannot be transferred
    fn transfer_from(
        &mut self,
        _from: Address,
        _to: Address,
        _token_id: U256,
    ) -> Result<(), SBTErrors> {
        Err(SBTErrors::NotTransferable(NotTransferable {}))
    }

    /// Disabled: SBTs cannot be safely transferred
    fn safe_transfer_from(
        &mut self,
        _from: Address,
        _to: Address,
        _token_id: U256,
    ) -> Result<(), SBTErrors> {
        Err(SBTErrors::NotTransferable(NotTransferable {}))
    }

    /// Disabled: SBTs cannot be safely transferred with data
    fn safe_transfer_from_with_data(
        &mut self,
        _from: Address,
        _to: Address,
        _token_id: U256,
        _data: Vec<u8>,
    ) -> Result<(), SBTErrors> {
        Err(SBTErrors::NotTransferable(NotTransferable {}))
    }

    /// Disabled: SBTs cannot be approved for transfer
    pub fn approve(&mut self, _to: Address, _token_id: U256) -> Result<(), SBTErrors> {
        Err(SBTErrors::NotTransferable(NotTransferable {}))
    }

    /// Disabled: SBTs cannot be approved for all
    pub fn set_approval_for_all(
        &mut self,
        _operator: Address,
        _approved: bool,
    ) -> Result<(), SBTErrors> {
        Err(SBTErrors::NotTransferable(NotTransferable {}))
    }
}
