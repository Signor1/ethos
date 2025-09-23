//!
//! # Soul Bound Token (SBT) Contract
//!
//! A non-transferable token that represents achievements, certifications, or reputation.
//! SBTs are permanently bound to an address and cannot be traded or transferred.
//!
//! ## Features
//! - Mint SBTs to addresses (only authorized issuers)
//! - View token metadata and ownership
//! - Revoke tokens when necessary
//! - Track token creation and management
//!
//! Note: This code is for educational purposes and has not been audited.
//!
// Allow `cargo stylus export-abi` to generate a main function.
#![cfg_attr(not(any(test, feature = "export-abi")), no_main)]
#![cfg_attr(not(any(test, feature = "export-abi")), no_std)]

#[macro_use]
extern crate alloc;

use alloc::{ string::String, vec::Vec };
use alloy_primitives::{ Address, U256 };
use stylus_sdk::{ alloy_sol_types::sol, prelude::* };

// Define persistent storage for the SBT contract
sol_storage! {
    #[entrypoint]
    pub struct SBT {
        /// Contract owner
        address owner;
        /// Next token ID to be minted
        uint256 next_token_id;
        /// Mapping from token ID to owner
        mapping(uint256 => address) token_owners;
        /// Mapping from owner to token count
        mapping(address => uint256) token_counts;
        /// Mapping from token ID to token URI
        mapping(uint256 => string) token_uris;
        /// Mapping to check if an address is an authorized issuer
        mapping(address => bool) authorized_issuers;
    }
}

// Define Solidity events and errors
sol! {
    /// Emitted when a new SBT is minted
    event SBTMinted(address indexed to, uint256 indexed token_id, string token_uri);
    
    /// Emitted when an SBT is revoked
    event SBTRevoked(address indexed from, uint256 indexed token_id);
    
    /// Emitted when an issuer is authorized
    event IssuerAuthorized(address indexed issuer);
    
    /// Emitted when an issuer is deauthorized
    event IssuerDeauthorized(address indexed issuer);
    
    /// Emitted when ownership is transferred
    event OwnershipTransferred(address indexed previous_owner, address indexed new_owner);
    
    // Custom error types
    error NotOwner();
    error NotAuthorizedIssuer();
    error TokenNotFound();
    error TokenNotOwnedBySender();
    error AddressZeroNotAllowed();
    error IssuerAlreadyAuthorized();
    error IssuerNotAuthorized();
    error EmptyTokenURI();
    error SelfRevocationNotAllowed();
}

/// Custom error enum for SBT contract
#[derive(SolidityError)]
pub enum SBTError {
    /// Caller is not the contract owner
    NotOwner(NotOwner),
    /// Caller is not an authorized issuer
    NotAuthorizedIssuer(NotAuthorizedIssuer),
    /// Requested token does not exist
    TokenNotFound(TokenNotFound),
    /// Token is not owned by the caller
    TokenNotOwnedBySender(TokenNotOwnedBySender),
    /// Zero address is not allowed for this operation
    AddressZeroNotAllowed(AddressZeroNotAllowed),
    /// Issuer is already authorized
    IssuerAlreadyAuthorized(IssuerAlreadyAuthorized),
    /// Issuer is not authorized
    IssuerNotAuthorized(IssuerNotAuthorized),
    /// Token URI cannot be empty
    EmptyTokenURI(EmptyTokenURI),
    /// Users cannot revoke their own tokens
    SelfRevocationNotAllowed(SelfRevocationNotAllowed),
}

/// Declare that `SBT` is a contract with the following external methods.
#[public]
impl SBT {
    /// Constructor - initializes the contract with the deployer as owner
    #[constructor]
    pub fn constructor(&mut self) -> Result<(), SBTError> {
        let owner = self.vm().tx_origin();

        if owner.is_zero() {
            return Err(SBTError::AddressZeroNotAllowed(AddressZeroNotAllowed {}));
        }

        self.owner.set(owner);
        self.next_token_id.set(U256::from(1));

        log(self.vm(), OwnershipTransferred {
            previous_owner: Address::ZERO,
            new_owner: owner,
        });

        Ok(())
    }

    /// Gets the owner of the contract
    pub fn owner(&self) -> Address {
        self.owner.get()
    }

    /// Gets the total number of tokens owned by an address
    ///
    /// # Arguments
    /// * `owner` - Address to check balance for
    ///
    /// # Returns
    /// The number of tokens owned by the address
    pub fn balance_of(&self, owner: Address) -> U256 {
        self.token_counts.get(owner)
    }

    /// Gets the owner of a specific token
    ///
    /// # Arguments
    /// * `token_id` - The token ID to check
    ///
    /// # Returns
    /// Result containing the owner address or TokenNotFound error
    ///
    /// # Errors
    /// * `TokenNotFound` - If the token ID doesn't exist
    pub fn owner_of(&self, token_id: U256) -> Result<Address, SBTError> {
        let owner = self.token_owners.get(token_id);
        if owner.is_zero() {
            return Err(SBTError::TokenNotFound(TokenNotFound {}));
        }
        Ok(owner)
    }

    /// Gets the URI for a specific token
    ///
    /// # Arguments
    /// * `token_id` - The token ID to get URI for
    ///
    /// # Returns
    /// Result containing the token URI or TokenNotFound error
    ///
    /// # Errors
    /// * `TokenNotFound` - If the token ID doesn't exist
    pub fn token_uri(&self, token_id: U256) -> Result<String, SBTError> {
        let owner = self.token_owners.get(token_id);
        if owner.is_zero() {
            return Err(SBTError::TokenNotFound(TokenNotFound {}));
        }
        Ok(self.token_uris.get(token_id).get_string())
    }

    /// Checks if an address is an authorized issuer
    ///
    /// # Arguments
    /// * `issuer` - Address to check authorization for
    ///
    /// # Returns
    /// True if the address is authorized to issue SBTs
    pub fn is_authorized_issuer(&self, issuer: Address) -> bool {
        issuer == self.owner.get() || self.authorized_issuers.get(issuer)
    }

    /// Authorizes an issuer to mint SBTs
    ///
    /// # Arguments
    /// * `issuer` - Address to authorize as issuer
    ///
    /// # Returns
    /// Result indicating success or error
    ///
    /// # Errors
    /// * `NotOwner` - If caller is not the contract owner
    /// * `AddressZeroNotAllowed` - If issuer address is zero
    /// * `IssuerAlreadyAuthorized` - If issuer is already authorized
    pub fn authorize_issuer(&mut self, issuer: Address) -> Result<(), SBTError> {
        if self.vm().msg_sender() != self.owner.get() {
            return Err(SBTError::NotOwner(NotOwner {}));
        }

        if issuer.is_zero() {
            return Err(SBTError::AddressZeroNotAllowed(AddressZeroNotAllowed {}));
        }

        if self.authorized_issuers.get(issuer) {
            return Err(SBTError::IssuerAlreadyAuthorized(IssuerAlreadyAuthorized {}));
        }

        self.authorized_issuers.insert(issuer, true);

        log(self.vm(), IssuerAuthorized { issuer });

        Ok(())
    }

    /// Deauthorizes an issuer from minting SBTs
    ///
    /// # Arguments
    /// * `issuer` - Address to deauthorize
    ///
    /// # Returns
    /// Result indicating success or error
    ///
    /// # Errors
    /// * `NotOwner` - If caller is not the contract owner
    /// * `IssuerNotAuthorized` - If issuer is not currently authorized
    pub fn deauthorize_issuer(&mut self, issuer: Address) -> Result<(), SBTError> {
        if self.vm().msg_sender() != self.owner.get() {
            return Err(SBTError::NotOwner(NotOwner {}));
        }

        if !self.authorized_issuers.get(issuer) {
            return Err(SBTError::IssuerNotAuthorized(IssuerNotAuthorized {}));
        }

        self.authorized_issuers.insert(issuer, false);

        log(self.vm(), IssuerDeauthorized { issuer });

        Ok(())
    }

    /// Mints a new SBT to the specified address
    ///
    /// # Arguments
    /// * `to` - Address to mint the token to
    /// * `token_uri` - URI containing token metadata
    ///
    /// # Returns
    /// Result containing the new token ID or error
    ///
    /// # Errors
    /// * `NotAuthorizedIssuer` - If caller is not authorized to issue SBTs
    /// * `AddressZeroNotAllowed` - If recipient address is zero
    /// * `EmptyTokenURI` - If token URI is empty
    pub fn mint(&mut self, to: Address, token_uri: String) -> Result<U256, SBTError> {
        let sender = self.vm().msg_sender();

        if !self.is_authorized_issuer(sender) {
            return Err(SBTError::NotAuthorizedIssuer(NotAuthorizedIssuer {}));
        }

        if to.is_zero() {
            return Err(SBTError::AddressZeroNotAllowed(AddressZeroNotAllowed {}));
        }

        if token_uri.is_empty() {
            return Err(SBTError::EmptyTokenURI(EmptyTokenURI {}));
        }

        let token_id = self.next_token_id.get();

        // Update storage
        self.token_owners.insert(token_id, to);
        self.token_uris.setter(token_id).set_str(&token_uri);
        self.token_counts.insert(to, self.token_counts.get(to) + U256::from(1));
        self.next_token_id.set(token_id + U256::from(1));

        log(self.vm(), SBTMinted {
            to,
            token_id,
            token_uri,
        });

        Ok(token_id)
    }

    /// Revokes an SBT from its owner
    ///
    /// # Arguments
    /// * `token_id` - ID of the token to revoke
    ///
    /// # Returns
    /// Result indicating success or error
    ///
    /// # Errors
    /// * `NotAuthorizedIssuer` - If caller is not authorized to revoke SBTs
    /// * `TokenNotFound` - If the token doesn't exist
    /// * `SelfRevocationNotAllowed` - If token owner tries to revoke their own token
    pub fn revoke(&mut self, token_id: U256) -> Result<(), SBTError> {
        let sender = self.vm().msg_sender();

        if !self.is_authorized_issuer(sender) {
            return Err(SBTError::NotAuthorizedIssuer(NotAuthorizedIssuer {}));
        }

        let token_owner = self.token_owners.get(token_id);
        if token_owner.is_zero() {
            return Err(SBTError::TokenNotFound(TokenNotFound {}));
        }

        // Prevent self-revocation (users cannot revoke their own tokens)
        if sender == token_owner {
            return Err(SBTError::SelfRevocationNotAllowed(SelfRevocationNotAllowed {}));
        }

        // Update storage
        self.token_owners.insert(token_id, Address::ZERO);
        self.token_uris.setter(token_id).set_str("");
        let current_balance = self.token_counts.get(token_owner);
        if current_balance > U256::ZERO {
            self.token_counts.insert(token_owner, current_balance - U256::from(1));
        }

        log(self.vm(), SBTRevoked {
            from: token_owner,
            token_id,
        });

        Ok(())
    }

    /// Gets the next token ID that will be minted
    pub fn next_token_id(&self) -> U256 {
        self.next_token_id.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sbt_contract() {
        use stylus_sdk::testing::*;
        let vm = TestVM::default();
        let mut contract = SBT::from(&vm);

        let owner = Address::from_word("owner");
        let user1 = Address::from_word("alice");
        let user2 = Address::from_word("bob");

        vm.set_sender(owner);

        // Test constructor
        let result = contract.constructor();
        assert!(result.is_ok());
        assert_eq!(contract.owner(), owner);

        // Test authorize issuer
        vm.set_sender(owner);
        let result = contract.authorize_issuer(user1);
        assert!(result.is_ok());
        assert!(contract.is_authorized_issuer(user1));

        // Test mint SBT
        vm.set_sender(user1);
        let token_uri = String::from("https://example.com/token/1");
        let result = contract.mint(user2, token_uri.clone());
        assert!(result.is_ok());
        let token_id = result.unwrap();
        assert_eq!(token_id, U256::from(1));

        // Test token ownership
        assert_eq!(contract.owner_of(token_id).unwrap(), user2);
        assert_eq!(contract.balance_of(user2), U256::from(1));
        assert_eq!(contract.token_uri(token_id).unwrap(), token_uri);

        // Test revoke token
        vm.set_sender(owner);
        let result = contract.revoke(token_id);
        assert!(result.is_ok());

        // Test error cases
        let result = contract.owner_of(token_id);
        assert!(matches!(result, Err(SBTError::TokenNotFound(_))));

        // Test unauthorized issuer error
        vm.set_sender(user2);
        let result = contract.mint(user1, String::from("test"));
        assert!(matches!(result, Err(SBTError::NotAuthorizedIssuer(_))));

        // Test empty URI error
        vm.set_sender(owner);
        let result = contract.mint(user1, String::new());
        assert!(matches!(result, Err(SBTError::EmptyTokenURI(_))));
    }
}
