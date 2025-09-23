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

sol! {
    // Errors
    error Unauthorized();
    error AccountAlreadyRegistered();
    error AddressZeroNotAllowed();
    error NotPendingOwner();

    //Events
    event IssuerRegistered(address indexed issuer);
    event NewOwnerRegistered(address indexed new_owner);
    event OwnershipTransferred(address indexed previous_owner, address indexed new_owner);
}

#[derive(SolidityError)]
pub enum IssuerRegistryError {
    Unauthorized(Unauthorized),
    AccountAlreadyRegistered(AccountAlreadyRegistered),
    AddressZeroNotAllowed(AddressZeroNotAllowed),
    NotPendingOwner(NotPendingOwner),
}

sol_storage! {
    #[entrypoint]
    pub struct IssuerRegistry {
        address owner;
        address pending_owner;
        mapping(address => bool) is_registered;
    }
}

#[public]
impl IssuerRegistry {
    #[constructor]
    fn constructor(&mut self) -> Result<(), IssuerRegistryError> {
        let owner: Address = self.vm().tx_origin();

        if owner.is_zero() {
            return Err(IssuerRegistryError::AddressZeroNotAllowed(
                AddressZeroNotAllowed {},
            ));
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

    fn register_as_issuer(&mut self) -> Result<(), IssuerRegistryError> {
        let issuer_address = self.vm().msg_sender();

        if issuer_address.is_zero() {
            return Err(IssuerRegistryError::AddressZeroNotAllowed(
                AddressZeroNotAllowed {},
            ));
        }

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

    fn transfer_ownership(&mut self, new_owner: Address) -> Result<(), IssuerRegistryError> {
        let current_owner = self.owner.get();
        if current_owner != self.vm().msg_sender() {
            return Err(IssuerRegistryError::Unauthorized(Unauthorized {}));
        }

        if new_owner.is_zero() {
            return Err(IssuerRegistryError::AddressZeroNotAllowed(
                AddressZeroNotAllowed {},
            ));
        }

        self.pending_owner.set(new_owner);

        log(self.vm(), NewOwnerRegistered { new_owner });

        Ok(())
    }

    fn accept_ownership(&mut self) -> Result<(), IssuerRegistryError> {
        let caller = self.vm().msg_sender();
        let pending_owner = self.pending_owner.get();

        if caller.is_zero() {
            return Err(IssuerRegistryError::AddressZeroNotAllowed(
                AddressZeroNotAllowed {},
            ));
        }

        if pending_owner != caller {
            return Err(IssuerRegistryError::NotPendingOwner(NotPendingOwner {}));
        }

        let previous_owner = self.owner.get();
        self.owner.set(pending_owner);
        self.pending_owner.set(Address::ZERO);

        log(
            self.vm(),
            OwnershipTransferred {
                previous_owner,
                new_owner: pending_owner,
            },
        );

        Ok(())
    }

    fn is_issuer(&self, issuer_address: Address) -> bool {
        self.is_registered.get(issuer_address)
    }

    fn get_owner(&self) -> Address {
        self.owner.get()
    }

    fn get_pending_owner(&self) -> Address {
        self.pending_owner.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::address;
    use stylus_sdk::testing::*;

    fn setup_contract() -> (TestVM, IssuerRegistry) {
        let vm = TestVM::default();
        let mut contract = IssuerRegistry::from(&vm);
        let owner = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");

        vm.set_sender(owner);
        let result = contract.constructor();
        assert!(result.is_ok());

        (vm, contract)
    }

    // CONSTRUCTOR TESTS

    #[test]
    fn test_constructor_success() {
        let (_vm, contract) = setup_contract();
        let owner = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");

        assert_eq!(contract.get_owner(), owner);
        assert_eq!(contract.get_pending_owner(), Address::ZERO);
    }

    #[test]
    fn test_constructor_with_zero_address() {
        let vm = TestVM::default();
        let mut contract = IssuerRegistry::from(&vm);

        // Set sender to zero address
        vm.set_sender(Address::ZERO);

        let result = contract.constructor();
        assert!(matches!(
            result,
            Err(IssuerRegistryError::AddressZeroNotAllowed(_))
        ));
    }

    // ISSUER REGISTRATION TESTS

    #[test]
    fn test_register_as_issuer_success() {
        let (vm, mut contract) = setup_contract();
        let alice = address!("70997970C51812dc3A010C7d01b50e0d17dc79C8");

        vm.set_sender(alice);
        let result = contract.register_as_issuer();

        assert!(result.is_ok());
        assert!(contract.is_issuer(alice));
    }

    #[test]
    fn test_register_multiple_issuers() {
        let (vm, mut contract) = setup_contract();
        let alice = address!("70997970C51812dc3A010C7d01b50e0d17dc79C8");
        let bob = address!("3C44CdDdB6a900fa2b585dd299e03d12FA4293BC");

        // Register Alice
        vm.set_sender(alice);
        assert!(contract.register_as_issuer().is_ok());

        // Register Bob
        vm.set_sender(bob);
        assert!(contract.register_as_issuer().is_ok());

        // Verify both are registered
        assert!(contract.is_issuer(alice));
        assert!(contract.is_issuer(bob));
    }

    #[test]
    fn test_register_as_issuer_duplicate() {
        let (vm, mut contract) = setup_contract();
        let alice = address!("70997970C51812dc3A010C7d01b50e0d17dc79C8");

        vm.set_sender(alice);
        assert!(contract.register_as_issuer().is_ok());

        // Try to register again
        let result = contract.register_as_issuer();
        assert!(matches!(
            result,
            Err(IssuerRegistryError::AccountAlreadyRegistered(_))
        ));
    }

    #[test]
    fn test_register_as_issuer_zero_address() {
        let (vm, mut contract) = setup_contract();

        vm.set_sender(Address::ZERO);
        let result = contract.register_as_issuer();

        assert!(matches!(
            result,
            Err(IssuerRegistryError::AddressZeroNotAllowed(_))
        ));
    }

    #[test]
    fn test_is_issuer_unregistered_address() {
        let (_vm, contract) = setup_contract();
        let alice = address!("70997970C51812dc3A010C7d01b50e0d17dc79C8");

        assert!(!contract.is_issuer(alice));
    }

    // OWNERSHIP TRANSFER TESTS

    #[test]
    fn test_transfer_ownership_success() {
        let (vm, mut contract) = setup_contract();
        let owner = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
        let alice = address!("70997970C51812dc3A010C7d01b50e0d17dc79C8");

        vm.set_sender(owner);
        let result = contract.transfer_ownership(alice);

        assert!(result.is_ok());
        assert_eq!(contract.get_pending_owner(), alice);
        // Owner should remain unchanged until accepted
        assert_eq!(contract.get_owner(), owner);
    }

    #[test]
    fn test_transfer_ownership_unauthorized() {
        let (vm, mut contract) = setup_contract();
        let alice = address!("70997970C51812dc3A010C7d01b50e0d17dc79C8");
        let bob = address!("3C44CdDdB6a900fa2b585dd299e03d12FA4293BC");

        // Alice (not owner) tries to transfer ownership
        vm.set_sender(alice);
        let result = contract.transfer_ownership(bob);

        assert!(matches!(result, Err(IssuerRegistryError::Unauthorized(_))));
    }

    #[test]
    fn test_transfer_ownership_to_zero_address() {
        let (vm, mut contract) = setup_contract();
        let owner = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");

        vm.set_sender(owner);
        let result = contract.transfer_ownership(Address::ZERO);

        assert!(matches!(
            result,
            Err(IssuerRegistryError::AddressZeroNotAllowed(_))
        ));
    }

    // OWNERSHIP ACCEPTANCE TESTS

    #[test]
    fn test_accept_ownership_success() {
        let (vm, mut contract) = setup_contract();
        let owner = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
        let alice = address!("70997970C51812dc3A010C7d01b50e0d17dc79C8");

        // Owner transfers ownership to Alice
        vm.set_sender(owner);
        assert!(contract.transfer_ownership(alice).is_ok());

        // Alice accepts ownership
        vm.set_sender(alice);
        let result = contract.accept_ownership();

        assert!(result.is_ok());
        assert_eq!(contract.get_owner(), alice);
        assert_eq!(contract.get_pending_owner(), Address::ZERO);
    }

    #[test]
    fn test_accept_ownership_not_pending_owner() {
        let (vm, mut contract) = setup_contract();
        let owner = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
        let alice = address!("70997970C51812dc3A010C7d01b50e0d17dc79C8");
        let bob = address!("3C44CdDdB6a900fa2b585dd299e03d12FA4293BC");

        // Owner transfers ownership to Alice
        vm.set_sender(owner);
        assert!(contract.transfer_ownership(alice).is_ok());

        // Bob (not pending owner) tries to accept
        vm.set_sender(bob);
        let result = contract.accept_ownership();

        assert!(matches!(
            result,
            Err(IssuerRegistryError::NotPendingOwner(_))
        ));
    }

    #[test]
    fn test_accept_ownership_no_pending_transfer() {
        let (vm, mut contract) = setup_contract();
        let alice = address!("70997970C51812dc3A010C7d01b50e0d17dc79C8");

        // Alice tries to accept ownership without any transfer initiated
        vm.set_sender(alice);
        let result = contract.accept_ownership();

        assert!(matches!(
            result,
            Err(IssuerRegistryError::NotPendingOwner(_))
        ));
    }

    #[test]
    fn test_accept_ownership_zero_address() {
        let (vm, mut contract) = setup_contract();

        vm.set_sender(Address::ZERO);
        let result = contract.accept_ownership();

        assert!(matches!(
            result,
            Err(IssuerRegistryError::AddressZeroNotAllowed(_))
        ));
    }
}
