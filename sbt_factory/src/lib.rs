//!
//! # Soul Bound Token (SBT) Factory Contract
//!
//! A factory contract for creating and managing multiple SBT contracts.
//! This contract allows authorized users to deploy new SBT contracts and keeps track of them.
//!
//! ## Features
//! - Deploy new SBT contracts
//! - Track deployed contracts
//! - Manage deployment permissions
//! - Query deployed contracts by deployer
//!
//! Note: This code is for educational purposes and has not been audited.
//!
// Allow `cargo stylus export-abi` to generate a main function.
#![cfg_attr(not(any(test, feature = "export-abi")), no_main)]
#![cfg_attr(not(any(test, feature = "export-abi")), no_std)]

#[macro_use]
extern crate alloc;

use alloc::vec::Vec;
use alloy_primitives::{ Address, U256 };
use stylus_sdk::{ alloy_sol_types::sol, prelude::* };

// Define persistent storage for the SBT Factory contract
sol_storage! {
    #[entrypoint]
    pub struct SBTFactory {
        /// Contract owner
        address owner;
        /// Total number of deployed contracts
        uint256 total_deployed;
        /// Mapping from deployer to their deployed contracts
        mapping(address => address[]) deployed_by;
        /// Mapping from contract address to deployer
        mapping(address => address) contract_deployers;
        /// Mapping to track if an address can deploy contracts
        mapping(address => bool) authorized_deployers;
        /// Array of all deployed contracts
        address[] all_contracts;
    }
}

// Define Solidity events and errors
sol! {
    /// Emitted when a new SBT contract is deployed
    event SBTContractDeployed(
        address indexed deployer,
        address indexed contract_address,
        uint256 indexed deployment_id
    );
    
    /// Emitted when a deployer is authorized
    event DeployerAuthorized(address indexed deployer);
    
    /// Emitted when a deployer is deauthorized
    event DeployerDeauthorized(address indexed deployer);
    
    /// Emitted when ownership is transferred
    event OwnershipTransferred(address indexed previous_owner, address indexed new_owner);
    
    // Custom error types
    error NotOwner();
    error NotAuthorizedDeployer();
    error AddressZeroNotAllowed();
    error DeployerAlreadyAuthorized();
    error DeployerNotAuthorized();
    error ContractNotFound();
    error DeploymentFailed();
    error InvalidIndex();
}

/// Custom error enum for SBT Factory contract
#[derive(SolidityError)]
pub enum SBTFactoryError {
    /// Caller is not the contract owner
    NotOwner(NotOwner),
    /// Caller is not an authorized deployer
    NotAuthorizedDeployer(NotAuthorizedDeployer),
    /// Zero address is not allowed for this operation
    AddressZeroNotAllowed(AddressZeroNotAllowed),
    /// Deployer is already authorized
    DeployerAlreadyAuthorized(DeployerAlreadyAuthorized),
    /// Deployer is not authorized
    DeployerNotAuthorized(DeployerNotAuthorized),
    /// Contract not found
    ContractNotFound(ContractNotFound),
    /// Deployment failed
    DeploymentFailed(DeploymentFailed),
    /// Invalid index provided
    InvalidIndex(InvalidIndex),
}

/// Declare that `SBTFactory` is a contract with the following external methods.
#[public]
impl SBTFactory {
    /// Constructor - initializes the contract with the deployer as owner
    #[constructor]
    pub fn constructor(&mut self) -> Result<(), SBTFactoryError> {
        let owner = self.vm().tx_origin();

        if owner.is_zero() {
            return Err(SBTFactoryError::AddressZeroNotAllowed(AddressZeroNotAllowed {}));
        }

        self.owner.set(owner);
        self.total_deployed.set(U256::ZERO);

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

    /// Gets the total number of deployed SBT contracts
    pub fn total_deployed(&self) -> U256 {
        self.total_deployed.get()
    }

    /// Checks if an address is an authorized deployer
    ///
    /// # Arguments
    /// * `deployer` - Address to check authorization for
    ///
    /// # Returns
    /// True if the address is authorized to deploy SBT contracts
    pub fn is_authorized_deployer(&self, deployer: Address) -> bool {
        deployer == self.owner.get() || self.authorized_deployers.get(deployer)
    }

    /// Authorizes a deployer to create SBT contracts
    ///
    /// # Arguments
    /// * `deployer` - Address to authorize as deployer
    ///
    /// # Returns
    /// Result indicating success or error
    ///
    /// # Errors
    /// * `NotOwner` - If caller is not the contract owner
    /// * `AddressZeroNotAllowed` - If deployer address is zero
    /// * `DeployerAlreadyAuthorized` - If deployer is already authorized
    pub fn authorize_deployer(&mut self, deployer: Address) -> Result<(), SBTFactoryError> {
        if self.vm().msg_sender() != self.owner.get() {
            return Err(SBTFactoryError::NotOwner(NotOwner {}));
        }

        if deployer.is_zero() {
            return Err(SBTFactoryError::AddressZeroNotAllowed(AddressZeroNotAllowed {}));
        }

        if self.authorized_deployers.get(deployer) {
            return Err(SBTFactoryError::DeployerAlreadyAuthorized(DeployerAlreadyAuthorized {}));
        }

        self.authorized_deployers.insert(deployer, true);

        log(self.vm(), DeployerAuthorized { deployer });

        Ok(())
    }

    /// Deauthorizes a deployer from creating SBT contracts
    ///
    /// # Arguments
    /// * `deployer` - Address to deauthorize
    ///
    /// # Returns
    /// Result indicating success or error
    ///
    /// # Errors
    /// * `NotOwner` - If caller is not the contract owner
    /// * `DeployerNotAuthorized` - If deployer is not currently authorized
    pub fn deauthorize_deployer(&mut self, deployer: Address) -> Result<(), SBTFactoryError> {
        if self.vm().msg_sender() != self.owner.get() {
            return Err(SBTFactoryError::NotOwner(NotOwner {}));
        }

        if !self.authorized_deployers.get(deployer) {
            return Err(SBTFactoryError::DeployerNotAuthorized(DeployerNotAuthorized {}));
        }

        self.authorized_deployers.insert(deployer, false);

        log(self.vm(), DeployerDeauthorized { deployer });

        Ok(())
    }

    /// Deploys a new SBT contract (simplified - would use CREATE2 in production)
    ///
    /// # Arguments
    /// * `salt` - Salt for deterministic deployment
    ///
    /// # Returns
    /// Result containing the new contract address or error
    ///
    /// # Errors
    /// * `NotAuthorizedDeployer` - If caller is not authorized to deploy contracts
    /// * `DeploymentFailed` - If contract deployment fails
    pub fn deploy_sbt_contract(&mut self, salt: U256) -> Result<Address, SBTFactoryError> {
        let deployer = self.vm().msg_sender();

        if !self.is_authorized_deployer(deployer) {
            return Err(SBTFactoryError::NotAuthorizedDeployer(NotAuthorizedDeployer {}));
        }

        // In a real implementation, this would use CREATE2 opcode to deploy the contract
        // For this example, we simulate the deployment with a deterministic address
        let contract_address = self.simulate_contract_deployment(deployer, salt);

        // Update storage
        let deployment_id = self.total_deployed.get();
        self.deployed_by.setter(deployer).push(contract_address);
        self.contract_deployers.insert(contract_address, deployer);
        self.all_contracts.push(contract_address);
        self.total_deployed.set(deployment_id + U256::from(1));

        log(self.vm(), SBTContractDeployed {
            deployer,
            contract_address,
            deployment_id,
        });

        Ok(contract_address)
    }

    /// Gets contracts deployed by a specific deployer
    ///
    /// # Arguments
    /// * `deployer` - Address of the deployer
    ///
    /// # Returns
    /// Vector of contract addresses deployed by the deployer
    pub fn get_contracts_by_deployer(&self, deployer: Address) -> Vec<Address> {
        let deployed_vec = self.deployed_by.getter(deployer);
        let mut result = Vec::new();
        for i in 0..deployed_vec.len() {
            result.push(deployed_vec.get(i).unwrap_or(Address::ZERO));
        }
        result
    }

    /// Gets the deployer of a specific contract
    ///
    /// # Arguments
    /// * `contract_address` - Address of the contract
    ///
    /// # Returns
    /// Result containing the deployer address or ContractNotFound error
    ///
    /// # Errors
    /// * `ContractNotFound` - If the contract was not deployed by this factory
    pub fn get_contract_deployer(
        &self,
        contract_address: Address
    ) -> Result<Address, SBTFactoryError> {
        let deployer = self.contract_deployers.get(contract_address);
        if deployer.is_zero() {
            return Err(SBTFactoryError::ContractNotFound(ContractNotFound {}));
        }
        Ok(deployer)
    }

    /// Gets a contract address by its deployment index
    ///
    /// # Arguments
    /// * `index` - Index of the deployment
    ///
    /// # Returns
    /// Result containing the contract address or InvalidIndex error
    ///
    /// # Errors
    /// * `InvalidIndex` - If the index is out of bounds
    pub fn get_contract_by_index(&self, index: U256) -> Result<Address, SBTFactoryError> {
        let total = self.total_deployed.get();
        if index >= total {
            return Err(SBTFactoryError::InvalidIndex(InvalidIndex {}));
        }

        // Convert U256 to usize for array indexing (simplified)
        let idx: usize = index.try_into().unwrap_or(0);
        Ok(self.all_contracts.get(idx).unwrap_or(Address::ZERO))
    }

    /// Gets all deployed contract addresses
    ///
    /// # Returns
    /// Vector of all deployed contract addresses
    pub fn get_all_contracts(&self) -> Vec<Address> {
        let mut result = Vec::new();
        for i in 0..self.all_contracts.len() {
            result.push(self.all_contracts.get(i).unwrap_or(Address::ZERO));
        }
        result
    }

    /// Simulates contract deployment for testing purposes
    /// In production, this would use CREATE2 opcode
    fn simulate_contract_deployment(&self, deployer: Address, salt: U256) -> Address {
        // Create a deterministic address based on deployer and salt
        // This is a simplified simulation - real deployment would use CREATE2
        let combined = format!("{:?}{:?}", deployer, salt);
        Address::from_slice(&combined.as_bytes()[0..20])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sbt_factory_contract() {
        use stylus_sdk::testing::*;
        let vm = TestVM::default();
        let mut contract = SBTFactory::from(&vm);

        let owner = Address::from_word("owner");
        let deployer1 = Address::from_word("alice");
        let deployer2 = Address::from_word("bob");

        vm.set_sender(owner);

        // Test constructor
        let result = contract.constructor();
        assert!(result.is_ok());
        assert_eq!(contract.owner(), owner);
        assert_eq!(contract.total_deployed(), U256::ZERO);

        // Test authorize deployer
        vm.set_sender(owner);
        let result = contract.authorize_deployer(deployer1);
        assert!(result.is_ok());
        assert!(contract.is_authorized_deployer(deployer1));

        // Test deploy contract
        vm.set_sender(deployer1);
        let salt = U256::from(123);
        let result = contract.deploy_sbt_contract(salt);
        assert!(result.is_ok());
        let contract_address = result.unwrap();

        assert_eq!(contract.total_deployed(), U256::from(1));
        assert_eq!(contract.get_contract_deployer(contract_address).unwrap(), deployer1);

        // Test get contracts by deployer
        let contracts = contract.get_contracts_by_deployer(deployer1);
        assert_eq!(contracts.len(), 1);
        assert_eq!(contracts[0], contract_address);

        // Test get contract by index
        let result = contract.get_contract_by_index(U256::ZERO);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), contract_address);

        // Test error cases
        let result = contract.get_contract_by_index(U256::from(999));
        assert!(matches!(result, Err(SBTFactoryError::InvalidIndex(_))));

        // Test unauthorized deployer error
        vm.set_sender(deployer2);
        let result = contract.deploy_sbt_contract(U256::from(456));
        assert!(matches!(result, Err(SBTFactoryError::NotAuthorizedDeployer(_))));

        // Test contract not found error
        let fake_address = Address::from_word("fake");
        let result = contract.get_contract_deployer(fake_address);
        assert!(matches!(result, Err(SBTFactoryError::ContractNotFound(_))));

        // Test deauthorize deployer
        vm.set_sender(owner);
        let result = contract.deauthorize_deployer(deployer1);
        assert!(result.is_ok());
        assert!(!contract.is_authorized_deployer(deployer1));
    }
}
