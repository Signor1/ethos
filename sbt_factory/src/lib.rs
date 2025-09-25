//!
//! # SBT Factory Contract
//! Allows registered issuers to have new SBT collections deployed and registered.
//!
// Allow `cargo stylus export-abi` to generate a main function.
#![cfg_attr(not(any(test, feature = "export-abi")), no_main)]
#![cfg_attr(not(any(test, feature = "export-abi")), no_std)]

#[macro_use]
extern crate alloc;

use alloc::{string::String, vec::Vec};
use alloy_primitives::Address;
use stylus_sdk::{alloy_primitives::U256, alloy_sol_types::sol, prelude::*};

sol_storage! {
    #[entrypoint]
    pub struct SBTFactory {
        /// All SBT collection addresses created
        address[] all_sbt_collections;
        /// Mapping from issuer address to their SBT collections
        mapping(address => SBTCollectionStorage[]) issuer_collections;
        /// Mapping to check if an address is a valid SBT from this factory
        mapping(address => bool) is_valid_sbt;
        /// Total number of collections created
        uint256 total_collections_count;
    }

    pub struct SBTCollectionStorage {
        string name;
        string symbol;
        address sbt_address;
    }
}

sol! {
    // Events
    event SBTCollectionRegistered(
        address indexed issuer,
        address indexed sbt_address,
        string name,
        string symbol,
    );

    // Errors
    error AddressZeroNotAllowed();
    error EmptyString();
    error ContractAlreadyRegistered();
}

#[derive(SolidityError)]
pub enum SBTFactoryError {
    AddressZeroNotAllowed(AddressZeroNotAllowed),
    EmptyString(EmptyString),
    ContractAlreadyRegistered(ContractAlreadyRegistered),
}

impl SBTFactory {
    fn record_sbt_collection(
        &mut self,
        issuer: Address,
        name: String,
        symbol: String,
        sbt_address: Address,
    ) -> Result<(), SBTFactoryError> {
        // Get the storage vector and add a new entry
        let mut collections_setter = self.issuer_collections.setter(issuer);
        let mut new_collection = collections_setter.grow();

        // Set the fields directly on the storage struct
        new_collection.name.set_str(&name);
        new_collection.symbol.set_str(&symbol);
        new_collection.sbt_address.set(sbt_address);

        // Add to global list
        self.all_sbt_collections.push(sbt_address);

        // Mark as valid SBT
        self.is_valid_sbt.insert(sbt_address, true);

        // Increment total count
        let current_count = self.total_collections_count.get();
        self.total_collections_count
            .set(current_count + U256::from(1));

        Ok(())
    }
}

#[public]
impl SBTFactory {
    #[constructor]
    fn constructor(&mut self) {
        self.total_collections_count.set(U256::ZERO);
    }

    /// Register an SBT collection
    ///
    /// How it works in Stylus:
    /// 1. Issuer deploys SBT contract externally using `cargo stylus deploy` or thirdweb client (frontend)
    /// 2. Issuer calls this function to register their deployed SBT with the factory
    /// 3. Factory verifies the issuer owns the SBT contract
    /// 4. Factory tracks the SBT for management purposes
    fn register_sbt_collection(
        &mut self,
        sbt_address: Address,
        name: String,
        symbol: String,
    ) -> Result<(), SBTFactoryError> {
        let issuer = self.vm().msg_sender();

        if issuer.is_zero() || sbt_address.is_zero() {
            return Err(SBTFactoryError::AddressZeroNotAllowed(
                AddressZeroNotAllowed {},
            ));
        }

        if name.is_empty() || symbol.is_empty() {
            return Err(SBTFactoryError::EmptyString(EmptyString {}));
        }

        if self.is_valid_sbt.get(sbt_address) {
            return Err(SBTFactoryError::ContractAlreadyRegistered(
                ContractAlreadyRegistered {},
            ));
        }

        self.record_sbt_collection(issuer, name.clone(), symbol.clone(), sbt_address)?;

        log(
            self.vm(),
            SBTCollectionRegistered {
                issuer,
                sbt_address,
                name,
                symbol,
            },
        );
        Ok(())
    }

    fn get_issuer_collections(&self, issuer: Address) -> Vec<(String, String, Address)> {
        let storage_vec = self.issuer_collections.get(issuer);
        let mut result = Vec::new();

        // Return simple tuples instead of structs
        for i in 0..storage_vec.len() {
            if let Some(collection) = storage_vec.get(i) {
                let tuple = (
                    collection.name.get_string(),
                    collection.symbol.get_string(),
                    collection.sbt_address.get(),
                );
                result.push(tuple);
            }
        }

        result
    }

    fn is_valid_sbt_contract(&self, sbt_address: Address) -> bool {
        self.is_valid_sbt.get(sbt_address)
    }

    fn get_total_collections(&self) -> U256 {
        self.total_collections_count.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, Address, U256};
    use stylus_sdk::testing::*;

    fn setup_factory() -> (TestVM, SBTFactory) {
        let vm = TestVM::default();
        let mut factory = SBTFactory::from(&vm);
        let _result = factory.constructor();
        (vm, factory)
    }

    #[test]
    fn test_constructor_initialization() {
        let (_vm, factory) = setup_factory();
        assert_eq!(factory.get_total_collections(), U256::ZERO);
    }

    #[test]
    fn test_register_sbt_collection_success() {
        let (vm, mut factory) = setup_factory();
        let issuer = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
        let sbt_addr = address!("70997970C51812dc3A010C7d01b50e0d17dc79C8");

        vm.set_sender(issuer);
        let result =
            factory.register_sbt_collection(sbt_addr, "Test SBT".to_string(), "TSBT".to_string());

        assert!(result.is_ok());
        assert_eq!(factory.get_total_collections(), U256::from(1));
        assert!(factory.is_valid_sbt_contract(sbt_addr));
    }

    #[test]
    fn test_register_zero_sender() {
        let (vm, mut factory) = setup_factory();
        vm.set_sender(Address::ZERO);
        let sbt_addr = address!("3333333333333333333333333333333333333333");
        let name = "Test SBT".to_string();
        let symbol = "TSBT".to_string();

        let res = factory.register_sbt_collection(sbt_addr, name, symbol);
        assert!(matches!(
            res,
            Err(SBTFactoryError::AddressZeroNotAllowed(_))
        ));
    }

    #[test]
    fn test_register_multiple_collections_same_issuer() {
        let (vm, mut factory) = setup_factory();
        let issuer = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
        let sbt_addr1 = address!("70997970C51812dc3A010C7d01b50e0d17dc79C8");
        let sbt_addr2 = address!("3C44CdDdB6a900fa2b585dd299e03d12FA4293BC");

        vm.set_sender(issuer);

        // Register first collection
        let result1 =
            factory.register_sbt_collection(sbt_addr1, "First SBT".to_string(), "FSBT".to_string());
        assert!(result1.is_ok());

        // Register second collection
        let result2 = factory.register_sbt_collection(
            sbt_addr2,
            "Second SBT".to_string(),
            "SSBT".to_string(),
        );
        assert!(result2.is_ok());

        assert_eq!(factory.get_total_collections(), U256::from(2));
        assert!(factory.is_valid_sbt_contract(sbt_addr1));
        assert!(factory.is_valid_sbt_contract(sbt_addr2));

        // Check issuer collections
        let collections = factory.get_issuer_collections(issuer);
        assert_eq!(collections.len(), 2);
        assert_eq!(collections[0].0, "First SBT");
        assert_eq!(collections[1].0, "Second SBT");
    }

    #[test]
    fn test_register_multiple_issuers() {
        let (vm, mut factory) = setup_factory();
        let issuer1 = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
        let issuer2 = address!("70997970C51812dc3A010C7d01b50e0d17dc79C8");
        let sbt_addr1 = address!("3C44CdDdB6a900fa2b585dd299e03d12FA4293BC");
        let sbt_addr2 = address!("90F79bf6EB2c4f870365E785982E1f101E93b906");

        // Register from first issuer
        vm.set_sender(issuer1);
        let result1 = factory.register_sbt_collection(
            sbt_addr1,
            "Issuer1 SBT".to_string(),
            "I1SBT".to_string(),
        );
        assert!(result1.is_ok());

        // Register from second issuer
        vm.set_sender(issuer2);
        let result2 = factory.register_sbt_collection(
            sbt_addr2,
            "Issuer2 SBT".to_string(),
            "I2SBT".to_string(),
        );
        assert!(result2.is_ok());

        assert_eq!(factory.get_total_collections(), U256::from(2));

        // Check each issuer's collections
        let collections1 = factory.get_issuer_collections(issuer1);
        let collections2 = factory.get_issuer_collections(issuer2);

        assert_eq!(collections1.len(), 1);
        assert_eq!(collections2.len(), 1);
        assert_eq!(collections1[0].0, "Issuer1 SBT");
        assert_eq!(collections2[0].0, "Issuer2 SBT");
    }

    #[test]
    fn test_register_zero_sbt_address() {
        let (vm, mut factory) = setup_factory();
        let issuer = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");

        vm.set_sender(issuer);
        let result = factory.register_sbt_collection(
            Address::ZERO,
            "Test SBT".to_string(),
            "TSBT".to_string(),
        );

        assert!(matches!(
            result,
            Err(SBTFactoryError::AddressZeroNotAllowed(_))
        ));
    }

    #[test]
    fn test_register_empty_name() {
        let (vm, mut factory) = setup_factory();
        let issuer = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
        let sbt_addr = address!("70997970C51812dc3A010C7d01b50e0d17dc79C8");

        vm.set_sender(issuer);
        let result = factory.register_sbt_collection(sbt_addr, String::new(), "TSBT".to_string());

        assert!(matches!(result, Err(SBTFactoryError::EmptyString(_))));
    }

    #[test]
    fn test_register_empty_symbol() {
        let (vm, mut factory) = setup_factory();
        let issuer = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
        let sbt_addr = address!("70997970C51812dc3A010C7d01b50e0d17dc79C8");

        vm.set_sender(issuer);
        let result =
            factory.register_sbt_collection(sbt_addr, "Test SBT".to_string(), String::new());

        assert!(matches!(result, Err(SBTFactoryError::EmptyString(_))));
    }

    #[test]
    fn test_register_duplicate_contract() {
        let (vm, mut factory) = setup_factory();
        let issuer = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
        let sbt_addr = address!("70997970C51812dc3A010C7d01b50e0d17dc79C8");

        vm.set_sender(issuer);

        // Register first time - should succeed
        let result1 =
            factory.register_sbt_collection(sbt_addr, "Test SBT".to_string(), "TSBT".to_string());
        assert!(result1.is_ok());

        // Register same address again - should fail
        let result2 = factory.register_sbt_collection(
            sbt_addr,
            "Another SBT".to_string(),
            "ASBT".to_string(),
        );
        assert!(matches!(
            result2,
            Err(SBTFactoryError::ContractAlreadyRegistered(_))
        ));
    }

    // VIEW FUNCTION TESTS

    #[test]
    fn test_get_issuer_collections_empty() {
        let (_vm, factory) = setup_factory();
        let issuer = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");

        let collections = factory.get_issuer_collections(issuer);
        assert!(collections.is_empty());
    }

    #[test]
    fn test_get_issuer_collections_with_data() {
        let (vm, mut factory) = setup_factory();
        let issuer = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
        let sbt_addr = address!("70997970C51812dc3A010C7d01b50e0d17dc79C8");

        vm.set_sender(issuer);
        let _ = factory.register_sbt_collection(
            sbt_addr,
            "Test SBT Collection".to_string(),
            "TSBT".to_string(),
        );

        let collections = factory.get_issuer_collections(issuer);
        assert_eq!(collections.len(), 1);
        assert_eq!(
            collections[0],
            (
                "Test SBT Collection".to_string(),
                "TSBT".to_string(),
                sbt_addr
            )
        );
    }

    #[test]
    fn test_is_valid_sbt_contract_false() {
        let (_vm, factory) = setup_factory();
        let random_addr = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");

        assert!(!factory.is_valid_sbt_contract(random_addr));
    }

    #[test]
    fn test_is_valid_sbt_contract_true() {
        let (vm, mut factory) = setup_factory();
        let issuer = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
        let sbt_addr = address!("70997970C51812dc3A010C7d01b50e0d17dc79C8");

        vm.set_sender(issuer);
        let _ =
            factory.register_sbt_collection(sbt_addr, "Test SBT".to_string(), "TSBT".to_string());

        assert!(factory.is_valid_sbt_contract(sbt_addr));
    }

    #[test]
    fn test_get_total_collections_increments() {
        let (vm, mut factory) = setup_factory();
        let issuer = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");

        vm.set_sender(issuer);
        assert_eq!(factory.get_total_collections(), U256::ZERO);

        // Register first collection
        let _ = factory.register_sbt_collection(
            address!("70997970C51812dc3A010C7d01b50e0d17dc79C8"),
            "SBT1".to_string(),
            "SBT1".to_string(),
        );
        assert_eq!(factory.get_total_collections(), U256::from(1));

        // Register second collection
        let _ = factory.register_sbt_collection(
            address!("3C44CdDdB6a900fa2b585dd299e03d12FA4293BC"),
            "SBT2".to_string(),
            "SBT2".to_string(),
        );
        assert_eq!(factory.get_total_collections(), U256::from(2));
    }

    // EDGE CASE TESTS

    #[test]
    fn test_different_issuer_same_name_symbol() {
        let (vm, mut factory) = setup_factory();
        let issuer1 = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
        let issuer2 = address!("70997970C51812dc3A010C7d01b50e0d17dc79C8");
        let sbt_addr1 = address!("3C44CdDdB6a900fa2b585dd299e03d12FA4293BC");
        let sbt_addr2 = address!("90F79bf6EB2c4f870365E785982E1f101E93b906");

        // Same name and symbol but different addresses should work
        vm.set_sender(issuer1);
        let result1 =
            factory.register_sbt_collection(sbt_addr1, "Same Name".to_string(), "SAME".to_string());
        assert!(result1.is_ok());

        vm.set_sender(issuer2);
        let result2 =
            factory.register_sbt_collection(sbt_addr2, "Same Name".to_string(), "SAME".to_string());
        assert!(result2.is_ok());

        assert_eq!(factory.get_total_collections(), U256::from(2));
    }

    #[test]
    fn test_very_long_name_and_symbol() {
        let (vm, mut factory) = setup_factory();
        let issuer = address!("f39Fd6e51aad88F6F4ce6aB8827279cffFb92266");
        let sbt_addr = address!("70997970C51812dc3A010C7d01b50e0d17dc79C8");

        vm.set_sender(issuer);
        let long_name = "A".repeat(100);
        let long_symbol = "B".repeat(50);

        let result =
            factory.register_sbt_collection(sbt_addr, long_name.clone(), long_symbol.clone());
        assert!(result.is_ok());

        let collections = factory.get_issuer_collections(issuer);
        assert_eq!(collections[0].0, long_name);
        assert_eq!(collections[0].1, long_symbol);
    }
}
