//! # Reputation System Interfaces
//!
//! Defines the common traits (interfaces) that allow the contracts
//! in the system to interact with each other without needing their full source code.

#![cfg_attr(not(feature = "export-abi"), no_main)]
extern crate alloc;

use alloc::vec::Vec;
use alloy_primitives::{Address, U256};
use stylus_sdk::stylus_proc::external;

/// Interface for the IssuerRegistry.
/// The SBTFactory uses this to check if a caller is authorized.
#[external]
pub trait IIssuerRegistry {
    /// Returns true if the given address is a registered issuer.
    fn is_issuer(&self, issuer_address: Address) -> Result<bool, Vec<u8>>;
}

/// Interface for the SBT (Soulbound Token).
/// The ReputationStaking contract uses this to verify token ownership.
#[external]
pub trait ISBT {
    /// Returns the owner of a given token ID.
    fn owner_of(&self, token_id: U256) -> Result<Address, Vec<u8>>;
}
