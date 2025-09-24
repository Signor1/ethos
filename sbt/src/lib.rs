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

use alloc::{string::String, vec::Vec};
use alloy_sol_types::SolValue;
use openzeppelin_stylus::token::erc721::{self, Erc721, IErc721};
use stylus_sdk::{
    alloy_primitives::{Address, FixedBytes, U256},
    alloy_sol_types::sol,
    crypto::keccak,
    prelude::*,
};

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

sol! {
    // Errors
    error NotTransferable();
    error Unauthorized();
    error TokenNotExists();
    error ZeroAddress();
    error EmptyArray();
}

#[derive(SolidityError)]
pub enum SBTErrors {
    // ERC721 errors
    InvalidOwner(erc721::ERC721InvalidOwner),
    NonexistentToken(erc721::ERC721NonexistentToken),
    IncorrectOwner(erc721::ERC721IncorrectOwner),
    InvalidSender(erc721::ERC721InvalidSender),
    InvalidReceiver(erc721::ERC721InvalidReceiver),
    InvalidReceiverWithReason(erc721::InvalidReceiverWithReason),
    InsufficientApproval(erc721::ERC721InsufficientApproval),
    InvalidApprover(erc721::ERC721InvalidApprover),
    InvalidOperator(erc721::ERC721InvalidOperator),
    // SBT specific errors
    NotTransferable(NotTransferable),
    Unauthorized(Unauthorized),
    TokenNotExists(TokenNotExists),
    ZeroAddress(ZeroAddress),
    EmptyArray(EmptyArray),
}

impl From<erc721::Error> for SBTErrors {
    fn from(value: erc721::Error) -> Self {
        match value {
            erc721::Error::IncorrectOwner(e) => SBTErrors::IncorrectOwner(e),
            erc721::Error::NonexistentToken(e) => SBTErrors::NonexistentToken(e),
            erc721::Error::InvalidOwner(e) => SBTErrors::InvalidOwner(e),
            erc721::Error::InvalidSender(e) => SBTErrors::InvalidSender(e),
            erc721::Error::InvalidReceiver(e) => SBTErrors::InvalidReceiver(e),
            erc721::Error::InvalidReceiverWithReason(e) => SBTErrors::InvalidReceiverWithReason(e),
            erc721::Error::InsufficientApproval(e) => SBTErrors::InsufficientApproval(e),
            erc721::Error::InvalidApprover(e) => SBTErrors::InvalidApprover(e),
            erc721::Error::InvalidOperator(e) => SBTErrors::InvalidOperator(e),
        }
    }
}

impl SBT {
    /// Generate deterministic entropy for each token
    fn generate_entropy(&self, token_id: U256, recipient: Address) -> FixedBytes<32> {
        let block_number = self.vm().block_number();
        let msg_sender = self.vm().msg_sender();
        let chain_id = self.vm().chain_id();

        let hash_data =
            (block_number, msg_sender, chain_id, token_id, recipient).abi_encode_sequence();
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

    /// Generate token URI with circular design
    #[selector(name = "tokenURI")]
    fn token_uri(&self, token_id: U256) -> Result<String, SBTErrors> {
        // Check if token exists by trying to get owner
        if self.erc721.owner_of(token_id).is_err() {
            return Err(SBTErrors::TokenNotExists(TokenNotExists {}));
        }

        let seed = self.entropy.get(token_id);
        let generator = generator::SBTGenerator::new(seed);
        let metadata = generator.metadata();
        Ok(metadata)
    }

    fn mint_to_one(&mut self, to: Address) -> Result<U256, SBTErrors> {
        // Only issuer can mint
        if self.vm().msg_sender() != self.issuer.get() {
            return Err(SBTErrors::Unauthorized(Unauthorized {}));
        }

        if to.is_zero() {
            return Err(SBTErrors::ZeroAddress(ZeroAddress {}));
        }

        let token_id = self.next_token_id.get();

        // Generate deterministic entropy for this token
        let seed = self.generate_entropy(token_id, to);
        self.entropy.setter(token_id).set(seed);

        // Mint via ERC721 base contract
        self.erc721._mint(to, token_id)?;

        // Increment next token ID
        self.next_token_id.set(token_id + U256::from(1));

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

            // Generate entropy for this token
            let seed = self.generate_entropy(current_token_id, *recipient);
            self.entropy.setter(current_token_id).set(seed);

            // Mint via ERC721 base contract
            self.erc721._mint(*recipient, current_token_id)?;

            token_ids.push(current_token_id);
            current_token_id += U256::from(1);
        }

        // Update next token ID
        self.next_token_id.set(current_token_id);

        Ok(token_ids)
    }

    fn get_issuer(&self) -> Address {
        self.issuer.get()
    }

    fn get_next_token_id(&self) -> U256 {
        self.next_token_id.get()
    }

    fn total_supply(&self) -> U256 {
        let next_id = self.next_token_id.get();
        if next_id == U256::from(1) {
            U256::ZERO
        } else {
            next_id - U256::from(1)
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::address;
    use stylus_sdk::testing::*;

    #[no_mangle]
    pub unsafe extern "C" fn emit_log(_pointer: *const u8, _len: usize, _: usize) {}

    fn setup_sbt() -> (TestVM, SBT) {
        let vm = TestVM::default();
        let mut sbt = SBT::from(&vm);

        let issuer = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
        vm.set_sender(issuer);

        let result = sbt.constructor("Test SBT".to_string(), "TSBT".to_string(), issuer);
        assert!(result.is_ok());

        (vm, sbt)
    }

    #[test]
    fn test_initialization() {
        let (_vm, sbt) = setup_sbt();
        assert_eq!(
            sbt.get_issuer(),
            address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266")
        );
        assert_eq!(sbt.get_next_token_id(), U256::from(1));
        assert_eq!(sbt.total_supply(), U256::ZERO);
        assert_eq!(sbt.name(), "Test SBT");
        assert_eq!(sbt.symbol(), "TSBT");
    }

    #[test]
    fn test_mint_to_one() {
        let (vm, mut sbt) = setup_sbt();
        let recipient = address!("70997970C51812dc3A010C7d01b50e0d17dc79C8");
        let issuer = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");

        vm.set_sender(issuer);

        let result = sbt.mint_to_one(recipient);
        assert!(result.is_ok());

        if let Ok(token_id) = result {
            assert_eq!(token_id, U256::from(1));
            assert!(sbt.erc721.owner_of(token_id).is_ok());
            assert_eq!(sbt.total_supply(), U256::from(1));
        }
    }

    #[test]
    fn test_unauthorized_mint() {
        let (vm, mut sbt) = setup_sbt();
        let recipient = address!("70997970C51812dc3A010C7d01b50e0d17dc79C8");
        let unauthorized = address!("3C44CdDdB6a900fa2b585dd299e03d12FA4293BC");

        vm.set_sender(unauthorized);

        let result = sbt.mint_to_one(recipient);
        assert!(matches!(result, Err(SBTErrors::Unauthorized(_))));
    }

    #[test]
    fn test_transfer_disabled() {
        let (vm, mut sbt) = setup_sbt();
        let recipient = address!("70997970C51812dc3A010C7d01b50e0d17dc79C8");
        let other = address!("3C44CdDdB6a900fa2b585dd299e03d12FA4293BC");
        let issuer = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");

        vm.set_sender(issuer);
        let result = sbt.mint_to_one(recipient);
        assert!(result.is_ok());

        if let Ok(token_id) = result {
            // Try to transfer - should fail
            let transfer_result = sbt.transfer_from(recipient, other, token_id);
            assert!(matches!(
                transfer_result,
                Err(SBTErrors::NotTransferable(_))
            ));

            // Try to approve - should fail
            let approve_result = sbt.approve(other, token_id);
            assert!(matches!(approve_result, Err(SBTErrors::NotTransferable(_))));
        }
    }

    #[test]
    fn test_token_uri_generation() {
        let (vm, mut sbt) = setup_sbt();
        let recipient = address!("70997970C51812dc3A010C7d01b50e0d17dc79C8");
        let issuer = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");

        vm.set_sender(issuer);
        let result = sbt.mint_to_one(recipient);
        assert!(result.is_ok());

        if let Ok(token_id) = result {
            let uri_result = sbt.token_uri(token_id);
            assert!(uri_result.is_ok());

            if let Ok(uri) = uri_result {
                assert!(uri.starts_with("data:application/json;base64,"));
            }
        }
    }
}
