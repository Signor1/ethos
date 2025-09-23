//!
//! # Issuer Registry Contract
//! Manages a whitelist of trusted addresses that are allowed to create SBTs.
//!
// Allow `cargo stylus export-abi` to generate a main function.
#![cfg_attr(not(any(test, feature = "export-abi")), no_main)]
#![cfg_attr(not(any(test, feature = "export-abi")), no_std)]

#[macro_use]
extern crate alloc;

use alloc::vec::Vec;
use alloy_primitives::Address;
use stylus_sdk::{alloy_sol_types::sol, prelude::*};

sol_storage! {
    #[entrypoint]
    pub struct IssuerRegistry {
        address owner;
        address new_owner;
        mapping(address => bool) is_registered;
    }
}

sol! {
    error Unauthorized();
    error AccountAlreadyRegistered();
    error AddressZeroNotAllowed();

    event IssuerRegistered(address indexed issuer);
    event NewOwnerRegistered(address indexed new_owner);
    event OwnershipTransferred(address indexed previous_owner, address indexed new_owner);
}

#[derive(SolidityError)]
pub enum IssuerRegistryError {
    Unauthorized(Unauthorized),
    AccountAlreadyRegistered(AccountAlreadyRegistered),
    AddressZeroNotAllowed(AddressZeroNotAllowed),
}

#[public]
impl IssuerRegistry {
    #[constructor]
    fn constructor(&mut self) -> Result<(), IssuerRegistryError> {
        let owner: Address = self.vm().tx_origin();

        if owner.is_zero() {
            return Err(IssuerRegistryError::Unauthorized(Unauthorized {}));
        }

        self.owner.set(owner);

        log(
            self.vm(),
            OwnershipTransferred {
                previous_owner: Address::ZERO,
                new_owner: owner,
            },
        );

        Ok(())
    }

    pub fn register_as_issuer(&mut self) -> Result<(), IssuerRegistryError> {
        let issuer_address = self.vm().msg_sender();
        if self.is_registered.get(issuer_address) {
            return Err(IssuerRegistryError::AccountAlreadyRegistered(
                AccountAlreadyRegistered {},
            ));
        }
        self.is_registered.insert(issuer_address, true);

        log(
            self.vm(),
            IssuerRegistered {
                issuer: issuer_address,
            },
        );

        Ok(())
    }

    pub fn transfer_ownership(&mut self, new_owner: Address) -> Result<(), IssuerRegistryError> {
        let current_owner = self.owner.get();
        if current_owner != self.vm().msg_sender() {
            return Err(IssuerRegistryError::Unauthorized(Unauthorized {}));
        }

        if new_owner.is_zero() {
            return Err(IssuerRegistryError::AddressZeroNotAllowed(
                AddressZeroNotAllowed {},
            ));
        }

        self.new_owner.set(new_owner);

        log(self.vm(), NewOwnerRegistered { new_owner });

        Ok(())
    }

    pub fn claim_ownership(&mut self) -> Result<(), IssuerRegistryError> {
        let caller = self.vm().msg_sender();
        if caller.is_zero() {
            return Err(IssuerRegistryError::AddressZeroNotAllowed(
                AddressZeroNotAllowed {},
            ));
        }

        let new_owner = self.new_owner.get();

        if new_owner != caller {
            return Err(IssuerRegistryError::Unauthorized(Unauthorized {}));
        }

        let previous_owner = self.owner.get();

        self.owner.set(new_owner);
        self.new_owner.set(Address::ZERO);

        log(
            self.vm(),
            OwnershipTransferred {
                previous_owner,
                new_owner,
            },
        );

        Ok(())
    }

    pub fn is_issuer(&self, issuer_address: Address) -> Result<bool, IssuerRegistryError> {
        Ok(self.is_registered.get(issuer_address))
    }
}

#[cfg(test)]
mod tests {
    use core::str::FromStr;

    use super::*;

    #[test]
    fn test_issuer_registry() {
        use stylus_sdk::testing::*;
        let vm = TestVM::default();
        let mut issuer_registry_contract = IssuerRegistry::from(&vm);

        let init_owner = Address::from_word("owner");
        vm.set_sender(init_owner);

        // initialize the issuer registry contract
        let contract_init = issuer_registry_contract.constructor()?;
        assert!(contract_init.is_ok());

        let sender_a = Address::from_word("alice");

        vm.set_sender(sender_a);
        // register as issuer
        let register_issuer = issuer_registry_contract.register_as_issuer();
        assert!(register_issuer.is_ok());

        let is_registered = issuer_registry_contract.is_issuer(sender_a);
        assert!(is_registered.unwrap());

        vm.set_sender(init_owner);
        // transfer ownership
        let transfer_ownership = issuer_registry_contract.transfer_ownership(sender_a);
        assert!(transfer_ownership.is_ok());

        vm.set_sender(sender_a);

        // claim ownership
        let claim_ownership = issuer_registry_contract.claim_ownership();
        assert!(claim_ownership.is_ok());
    }
}
