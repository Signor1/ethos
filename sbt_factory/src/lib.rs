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

// Interface for interacting with IssuerRegistry
sol_interface! {
    interface IIssuerRegistry {
        function is_issuer(address issuer_address) external view returns (bool);
    }
}

sol_interface! {
    interface Isbt {
        function get_issuer() external view returns (address);
    }
}

sol_storage! {
    #[entrypoint]
    pub struct SBTFactory {
        /// The address of the IssuerRegistry contract
        address issuer_registry_address;
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
    error CallerNotIssuer();
    error EmptyString();
    error SBTNotDeployedByIssuer();
    error ContractAlreadyRegistered();
}

#[derive(SolidityError)]
pub enum SBTFactoryError {
    AddressZeroNotAllowed(AddressZeroNotAllowed),
    CallerNotIssuer(CallerNotIssuer),
    EmptyString(EmptyString),
    SBTNotDeployedByIssuer(SBTNotDeployedByIssuer),
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
    fn constructor(&mut self, registry_address: Address) -> Result<(), SBTFactoryError> {
        if registry_address.is_zero() {
            return Err(SBTFactoryError::AddressZeroNotAllowed(
                AddressZeroNotAllowed {},
            ));
        }

        self.issuer_registry_address.set(registry_address);
        self.total_collections_count.set(U256::ZERO);
        Ok(())
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

        let registry = IIssuerRegistry::new(self.issuer_registry_address.get());
        match registry.is_issuer(&mut *self, issuer) {
            Ok(is_issuer) => {
                if !is_issuer {
                    return Err(SBTFactoryError::CallerNotIssuer(CallerNotIssuer {}));
                }
            }
            Err(_) => return Err(SBTFactoryError::CallerNotIssuer(CallerNotIssuer {})),
        }

        let sbt = Isbt::new(sbt_address);
        match sbt.get_issuer(&mut *self) {
            Ok(sbt_issuer) => {
                if sbt_issuer != issuer {
                    return Err(SBTFactoryError::SBTNotDeployedByIssuer(
                        SBTNotDeployedByIssuer {},
                    ));
                }
            }
            Err(_) => {
                return Err(SBTFactoryError::SBTNotDeployedByIssuer(
                    SBTNotDeployedByIssuer {},
                ))
            }
        }

        // Record the collection
        self.record_sbt_collection(issuer, name.clone(), symbol.clone(), sbt_address)?;

        // Emit event
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

    fn get_issuer_registry(&self) -> Address {
        self.issuer_registry_address.get()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloy_primitives::{address, Address, U256};
    use stylus_sdk::{crypto::keccak, testing::*};

    fn setup_factory() -> (TestVM, SBTFactory) {
        let vm = TestVM::default();
        let mut factory = SBTFactory::from(&vm);

        let registry_addr = address!("1111111111111111111111111111111111111111");

        let result = factory.constructor(registry_addr);
        assert!(result.is_ok());

        (vm, factory)
    }

    fn mock_is_issuer(vm: &mut TestVM, registry_addr: Address, issuer: Address, is_issuer: bool) {
        // Compute selector for is_issuer(address) -> keccak256("is_issuer(address)")
        let selector = keccak("is_issuer(address)".as_bytes())[0..4].to_vec();
        let mut calldata: Vec<u8> = selector;
        let mut padded_address = [0u8; 32];
        padded_address[12..32].copy_from_slice(&issuer.into_array());
        calldata.extend_from_slice(&padded_address);

        // Return true (1) or false (0) as a 32-byte value
        let ret_val = if is_issuer { U256::from(1) } else { U256::ZERO };
        let ret_data = ret_val.to_be_bytes::<32>().to_vec();

        vm.mock_call(registry_addr, calldata, Ok(ret_data));
    }

    fn mock_get_issuer(vm: &mut TestVM, sbt_addr: Address, returned_issuer: Address) {
        // Compute selector for get_issuer() -> keccak256("get_issuer()")
        let selector = keccak("get_issuer()".as_bytes())[0..4].to_vec();
        let calldata: Vec<u8> = selector;

        // Return address as 32-byte value (left-padded with zeros)
        let mut padded_address = [0u8; 32];
        padded_address[12..32].copy_from_slice(&returned_issuer.into_array());
        let ret_data = padded_address.to_vec();

        vm.mock_call(sbt_addr, calldata, Ok(ret_data));
    }

    #[test]
    fn test_initialization() {
        let (_vm, factory) = setup_factory();

        assert_eq!(
            factory.get_issuer_registry(),
            address!("1111111111111111111111111111111111111111")
        );
        assert_eq!(factory.get_total_collections(), U256::ZERO);
    }

    #[test]
    fn test_empty_collections_list() {
        let (_vm, factory) = setup_factory();
        let issuer = address!("3333333333333333333333333333333333333333");

        let collections = factory.get_issuer_collections(issuer);
        assert!(collections.is_empty());
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
    fn test_register_zero_sbt_address() {
        let (vm, mut factory) = setup_factory();
        let issuer = address!("2222222222222222222222222222222222222222");
        vm.set_sender(issuer);
        let name = "Test SBT".to_string();
        let symbol = "TSBT".to_string();

        let res = factory.register_sbt_collection(Address::ZERO, name, symbol);
        assert!(matches!(
            res,
            Err(SBTFactoryError::AddressZeroNotAllowed(_))
        ));
    }

    #[test]
    fn test_register_empty_name() {
        let (mut vm, mut factory) = setup_factory();
        let issuer = address!("2222222222222222222222222222222222222222");
        vm.set_sender(issuer);
        let sbt_addr = address!("3333333333333333333333333333333333333333");
        let symbol = "TSBT".to_string();

        let registry_addr = factory.get_issuer_registry();
        mock_is_issuer(&mut vm, registry_addr, issuer, true);
        mock_get_issuer(&mut vm, sbt_addr, issuer);

        let res = factory.register_sbt_collection(sbt_addr, String::new(), symbol);
        assert!(matches!(res, Err(SBTFactoryError::EmptyString(_))));
    }

    #[test]
    fn test_register_empty_symbol() {
        let (mut vm, mut factory) = setup_factory();
        let issuer = address!("2222222222222222222222222222222222222222");
        vm.set_sender(issuer);
        let sbt_addr = address!("3333333333333333333333333333333333333333");
        let name = "Test SBT".to_string();

        let registry_addr = factory.get_issuer_registry();
        mock_is_issuer(&mut vm, registry_addr, issuer, true);
        mock_get_issuer(&mut vm, sbt_addr, issuer);

        let res = factory.register_sbt_collection(sbt_addr, name, String::new());
        assert!(matches!(res, Err(SBTFactoryError::EmptyString(_))));
    }
}
