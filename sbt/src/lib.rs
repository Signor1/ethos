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
use stylus_sdk::{
    alloy_primitives::{Address, FixedBytes, U256},
    alloy_sol_types::sol,
    crypto::keccak,
    prelude::*,
};

sol_storage! {
    #[entrypoint]
    pub struct SBT {
        string name;
        string symbol;
        address issuer;
        uint256 next_token_id;
        mapping(uint256 => address) owners;
        mapping(address => uint256) balances;
        mapping(uint256 => bytes32) entropy;
    }
}

sol! {
    // Events
    event Transfer(address indexed from, address indexed to, uint256 indexed tokenId);
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

    /// Internal function to check if token exists
    fn token_exists(&self, token_id: U256) -> bool {
        !self.owners.get(token_id).is_zero()
    }
}

#[public]
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

    /// Returns the number of tokens in account's wallet
    #[selector(name = "balanceOf")]
    fn balance_of(&self, owner: Address) -> U256 {
        if owner.is_zero() {
            return U256::ZERO;
        }
        self.balances.get(owner)
    }

    /// Returns the owner of the token_id token
    #[selector(name = "ownerOf")]
    fn owner_of(&self, token_id: U256) -> Result<Address, SBTErrors> {
        let owner = self.owners.get(token_id);
        if owner.is_zero() {
            return Err(SBTErrors::TokenNotExists(TokenNotExists {}));
        }
        Ok(owner)
    }

    /// Generate token URI with circular design
    #[selector(name = "tokenURI")]
    fn token_uri(&self, token_id: U256) -> Result<String, SBTErrors> {
        if !self.token_exists(token_id) {
            return Err(SBTErrors::TokenNotExists(TokenNotExists {}));
        }
        let seed = self.entropy.get(token_id);
        let generator = generator::SBTGenerator::new(seed);
        Ok(generator.metadata())
    }

    fn mint_to_one(&mut self, to: Address) -> Result<U256, SBTErrors> {
        if self.vm().msg_sender() != self.issuer.get() {
            return Err(SBTErrors::Unauthorized(Unauthorized {}));
        }
        if to.is_zero() {
            return Err(SBTErrors::ZeroAddress(ZeroAddress {}));
        }
        let token_id = self.next_token_id.get();
        let seed = self.generate_entropy(token_id, to);
        self.entropy.setter(token_id).set(seed);
        self.owners.insert(token_id, to);
        let current_balance = self.balances.get(to);
        self.balances.insert(to, current_balance + U256::from(1));
        self.next_token_id.set(token_id + U256::from(1));

        log(
            self.vm(),
            Transfer {
                from: Address::ZERO,
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

            // Generate entropy for this token
            let seed = self.generate_entropy(current_token_id, *recipient);
            self.entropy.setter(current_token_id).set(seed);
            self.owners.insert(current_token_id, *recipient);

            let current_balance = self.balances.get(*recipient);
            self.balances
                .insert(*recipient, current_balance + U256::from(1));

            log(
                self.vm(),
                Transfer {
                    from: Address::ZERO,
                    to: *recipient,
                    tokenId: current_token_id,
                },
            );

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

    /// Check if interface is supported (minimal ERC165 implementation)
    #[selector(name = "supportsInterface")]
    fn supports_interface(&self, interface_id: FixedBytes<4>) -> bool {
        // ERC721 interface ID: 0x80ac58cd
        // ERC165 interface ID: 0x01ffc9a7
        interface_id == FixedBytes([0x80, 0xac, 0x58, 0xcd]) || // ERC721
            interface_id == FixedBytes([0x01, 0xff, 0xc9, 0xa7]) // ERC165
    }

    ///#######################################################
    /// DISABLED TRANSFER FUNCTIONS (Soulbound implementation)
    ///########################################################

    /// Disabled: SBTs cannot be transferred
    #[selector(name = "transferFrom")]
    fn transfer_from(
        &mut self,
        _from: Address,
        _to: Address,
        _token_id: U256,
    ) -> Result<(), SBTErrors> {
        Err(SBTErrors::NotTransferable(NotTransferable {}))
    }

    /// Disabled: SBTs cannot be approved for transfer
    pub fn approve(&mut self, _to: Address, _token_id: U256) -> Result<(), SBTErrors> {
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
    fn test_mint_and_balance() {
        let (vm, mut sbt) = setup_sbt();
        let recipient = address!("70997970C51812dc3A010C7d01b50e0d17dc79C8");
        let issuer = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");

        vm.set_sender(issuer);

        let result = sbt.mint_to_one(recipient);
        assert!(result.is_ok());

        if let Ok(token_id) = result {
            assert_eq!(token_id, U256::from(1));
            assert_eq!(sbt.balance_of(recipient), U256::from(1));
            assert!(sbt.owner_of(token_id).is_ok());
            assert_eq!(sbt.total_supply(), U256::from(1));
            if let Ok(addr) = sbt.owner_of(token_id) {
                assert_eq!(addr, recipient);
            }
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

    #[test]
    fn test_supports_interface() {
        let (_vm, sbt) = setup_sbt();

        // Test ERC721 interface
        let erc721_id = FixedBytes([0x80, 0xac, 0x58, 0xcd]);
        assert!(sbt.supports_interface(erc721_id));

        // Test ERC165 interface
        let erc165_id = FixedBytes([0x01, 0xff, 0xc9, 0xa7]);
        assert!(sbt.supports_interface(erc165_id));

        // Test unsupported interface
        let random_id = FixedBytes([0x12, 0x34, 0x56, 0x78]);
        assert!(!sbt.supports_interface(random_id));
    }
}
