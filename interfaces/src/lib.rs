//! # Reputation System Interfaces
//!
//! Defines the common traits (interfaces) that allow the contracts
//! in the system to interact with each other without needing their full source code.

#![cfg_attr(not(feature = "export-abi"), no_main)]
#![cfg_attr(not(feature = "export-abi"), no_std)]
extern crate alloc;

use stylus_sdk::alloy_sol_types::sol;

// Define Solidity-compatible interfaces
sol! {
    interface IIssuerRegistry {
        function is_issuer(address issuer_address) external view returns (bool);
    }
    
    interface ISBT {
        function owner_of(uint256 token_id) external view returns (address);
        function balance_of(address owner) external view returns (uint256);
    }
    
    interface ISBTFactory {
        function deploy_sbt_contract(uint256 salt) external returns (address);
        function is_authorized_deployer(address deployer) external view returns (bool);
    }
    
    interface IReputationStaking {
        function stake_of(address staker) external view returns (uint256);
        function reputation_of(address staker) external view returns (uint256);
    }
}

// Re-export the interfaces for easier use
pub use IIssuerRegistry::*;
pub use ISBT::*;
pub use ISBTFactory::*;
pub use IReputationStaking::*;
