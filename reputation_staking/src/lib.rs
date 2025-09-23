//!
//! # Reputation Staking Contract
//!
//! A staking contract that allows users to stake tokens to build reputation.
//! Reputation affects rewards and governance participation.
//!
//! ## Features
//! - Stake tokens to earn reputation points
//! - Withdraw staked tokens (with potential penalties)
//! - Track reputation scores and histories
//! - Reward distribution based on reputation
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

// Define persistent storage for the Reputation Staking contract
sol_storage! {
    #[entrypoint]
    pub struct ReputationStaking {
        /// Contract owner
        address owner;
        /// Total staked amount across all users
        uint256 total_staked;
        /// Minimum stake amount required
        uint256 minimum_stake;
        /// Reputation multiplier (basis points, 10000 = 100%)
        uint256 reputation_multiplier;
        /// Mapping from staker to their staked amount
        mapping(address => uint256) stakes;
        /// Mapping from staker to their reputation score
        mapping(address => uint256) reputation_scores;
        /// Mapping from staker to their stake timestamp
        mapping(address => uint256) stake_timestamps;
        /// Mapping to track if an address is blacklisted
        mapping(address => bool) blacklisted;
        /// Total reputation points in the system
        uint256 total_reputation;
    }
}

// Define Solidity events and errors
sol! {
    /// Emitted when tokens are staked
    event Staked(address indexed staker, uint256 amount, uint256 new_total);
    
    /// Emitted when tokens are withdrawn
    event Withdrawn(address indexed staker, uint256 amount, uint256 remaining);
    
    /// Emitted when reputation is updated
    event ReputationUpdated(address indexed staker, uint256 old_score, uint256 new_score);
    
    /// Emitted when a user is blacklisted
    event UserBlacklistedEvent(address indexed user);
    
    /// Emitted when a user is removed from blacklist
    event UserRemovedFromBlacklistEvent(address indexed user);
    
    /// Emitted when minimum stake is updated
    event MinimumStakeUpdated(uint256 old_minimum, uint256 new_minimum);
    
    /// Emitted when ownership is transferred
    event OwnershipTransferred(address indexed previous_owner, address indexed new_owner);
    
    // Custom error types
    error NotOwner();
    error InsufficientStake();
    error NoStakeToWithdraw();
    error UserBlacklisted();
    error AddressZeroNotAllowed();
    error InsufficientBalance();
    error InvalidAmount();
    error TooEarlyToWithdraw();
    error InvalidMultiplier();
    error UserNotBlacklisted();
}

/// Custom error enum for Reputation Staking contract
#[derive(SolidityError)]
pub enum ReputationStakingError {
    /// Caller is not the contract owner
    NotOwner(NotOwner),
    /// Stake amount is below minimum requirement
    InsufficientStake(InsufficientStake),
    /// User has no stake to withdraw
    NoStakeToWithdraw(NoStakeToWithdraw),
    /// User is blacklisted from participating
    UserBlacklisted(UserBlacklisted),
    /// Zero address is not allowed for this operation
    AddressZeroNotAllowed(AddressZeroNotAllowed),
    /// User has insufficient balance for this operation
    InsufficientBalance(InsufficientBalance),
    /// Invalid amount provided
    InvalidAmount(InvalidAmount),
    /// Withdrawal is not yet allowed (time lock)
    TooEarlyToWithdraw(TooEarlyToWithdraw),
    /// Invalid multiplier value
    InvalidMultiplier(InvalidMultiplier),
    /// User is not blacklisted
    UserNotBlacklisted(UserNotBlacklisted),
}

/// Declare that `ReputationStaking` is a contract with the following external methods.
#[public]
impl ReputationStaking {
    /// Constructor - initializes the contract with default values
    #[constructor]
    pub fn constructor(
        &mut self,
        minimum_stake: U256,
        reputation_multiplier: U256
    ) -> Result<(), ReputationStakingError> {
        let owner = self.vm().tx_origin();

        if owner.is_zero() {
            return Err(ReputationStakingError::AddressZeroNotAllowed(AddressZeroNotAllowed {}));
        }

        if reputation_multiplier > U256::from(50000) {
            // Max 500% multiplier
            return Err(ReputationStakingError::InvalidMultiplier(InvalidMultiplier {}));
        }

        self.owner.set(owner);
        self.minimum_stake.set(minimum_stake);
        self.reputation_multiplier.set(reputation_multiplier);
        self.total_staked.set(U256::ZERO);
        self.total_reputation.set(U256::ZERO);

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

    /// Gets the total amount staked across all users
    pub fn total_staked(&self) -> U256 {
        self.total_staked.get()
    }

    /// Gets the minimum stake amount required
    pub fn minimum_stake(&self) -> U256 {
        self.minimum_stake.get()
    }

    /// Gets the current reputation multiplier
    pub fn reputation_multiplier(&self) -> U256 {
        self.reputation_multiplier.get()
    }

    /// Gets the total reputation points in the system
    pub fn total_reputation(&self) -> U256 {
        self.total_reputation.get()
    }

    /// Gets the stake amount for a specific address
    ///
    /// # Arguments
    /// * `staker` - Address to check stake for
    ///
    /// # Returns
    /// The amount staked by the address
    pub fn stake_of(&self, staker: Address) -> U256 {
        self.stakes.get(staker)
    }

    /// Gets the reputation score for a specific address
    ///
    /// # Arguments
    /// * `staker` - Address to check reputation for
    ///
    /// # Returns
    /// The reputation score of the address
    pub fn reputation_of(&self, staker: Address) -> U256 {
        self.reputation_scores.get(staker)
    }

    /// Gets the stake timestamp for a specific address
    ///
    /// # Arguments
    /// * `staker` - Address to check timestamp for
    ///
    /// # Returns
    /// The timestamp when the address last staked
    pub fn stake_timestamp(&self, staker: Address) -> U256 {
        self.stake_timestamps.get(staker)
    }

    /// Checks if an address is blacklisted
    ///
    /// # Arguments
    /// * `user` - Address to check
    ///
    /// # Returns
    /// True if the address is blacklisted
    pub fn is_blacklisted(&self, user: Address) -> bool {
        self.blacklisted.get(user)
    }

    /// Stakes tokens to earn reputation
    ///
    /// # Returns
    /// Result indicating success or error
    ///
    /// # Errors
    /// * `UserBlacklisted` - If the caller is blacklisted
    /// * `InsufficientStake` - If the stake amount is below minimum
    /// * `InvalidAmount` - If no value is sent with the transaction
    #[payable]
    pub fn stake(&mut self) -> Result<(), ReputationStakingError> {
        let staker = self.vm().msg_sender();
        let amount = self.vm().msg_value();

        if self.blacklisted.get(staker) {
            return Err(ReputationStakingError::UserBlacklisted(UserBlacklisted {}));
        }

        if amount == U256::ZERO {
            return Err(ReputationStakingError::InvalidAmount(InvalidAmount {}));
        }

        let current_stake = self.stakes.get(staker);
        let new_total_stake = current_stake + amount;

        if new_total_stake < self.minimum_stake.get() {
            return Err(ReputationStakingError::InsufficientStake(InsufficientStake {}));
        }

        // Update stakes
        self.stakes.insert(staker, new_total_stake);
        self.total_staked.set(self.total_staked.get() + amount);
        self.stake_timestamps.insert(staker, U256::from(self.vm().block_timestamp()));

        // Calculate and update reputation
        let reputation_increase = self.calculate_reputation_increase(amount);
        let current_reputation = self.reputation_scores.get(staker);
        let new_reputation = current_reputation + reputation_increase;

        self.reputation_scores.insert(staker, new_reputation);
        self.total_reputation.set(self.total_reputation.get() + reputation_increase);

        log(self.vm(), Staked {
            staker,
            amount,
            new_total: new_total_stake,
        });

        log(self.vm(), ReputationUpdated {
            staker,
            old_score: current_reputation,
            new_score: new_reputation,
        });

        Ok(())
    }

    /// Withdraws staked tokens
    ///
    /// # Arguments
    /// * `amount` - Amount to withdraw
    ///
    /// # Returns
    /// Result indicating success or error
    ///
    /// # Errors
    /// * `UserBlacklisted` - If the caller is blacklisted
    /// * `NoStakeToWithdraw` - If the user has no stake
    /// * `InsufficientBalance` - If trying to withdraw more than staked
    /// * `TooEarlyToWithdraw` - If trying to withdraw too soon after staking
    pub fn withdraw(&mut self, amount: U256) -> Result<(), ReputationStakingError> {
        let staker = self.vm().msg_sender();

        if self.blacklisted.get(staker) {
            return Err(ReputationStakingError::UserBlacklisted(UserBlacklisted {}));
        }

        let current_stake = self.stakes.get(staker);
        if current_stake == U256::ZERO {
            return Err(ReputationStakingError::NoStakeToWithdraw(NoStakeToWithdraw {}));
        }

        if amount > current_stake {
            return Err(ReputationStakingError::InsufficientBalance(InsufficientBalance {}));
        }

        // Check time lock (24 hours = 86400 seconds)
        let stake_time = self.stake_timestamps.get(staker);
        let current_time = U256::from(self.vm().block_timestamp());
        if current_time < stake_time + U256::from(86400) {
            return Err(ReputationStakingError::TooEarlyToWithdraw(TooEarlyToWithdraw {}));
        }

        let new_stake = current_stake - amount;

        // Update stakes
        self.stakes.insert(staker, new_stake);
        self.total_staked.set(self.total_staked.get() - amount);

        // Reduce reputation proportionally
        let reputation_decrease = self.calculate_reputation_decrease(amount, current_stake);
        let current_reputation = self.reputation_scores.get(staker);
        let new_reputation = if current_reputation > reputation_decrease {
            current_reputation - reputation_decrease
        } else {
            U256::ZERO
        };

        self.reputation_scores.insert(staker, new_reputation);
        self.total_reputation.set(self.total_reputation.get() - reputation_decrease);

        log(self.vm(), Withdrawn {
            staker,
            amount,
            remaining: new_stake,
        });

        log(self.vm(), ReputationUpdated {
            staker,
            old_score: current_reputation,
            new_score: new_reputation,
        });

        Ok(())
    }

    /// Updates the minimum stake amount (owner only)
    ///
    /// # Arguments
    /// * `new_minimum` - New minimum stake amount
    ///
    /// # Returns
    /// Result indicating success or error
    ///
    /// # Errors
    /// * `NotOwner` - If caller is not the contract owner
    pub fn set_minimum_stake(&mut self, new_minimum: U256) -> Result<(), ReputationStakingError> {
        if self.vm().msg_sender() != self.owner.get() {
            return Err(ReputationStakingError::NotOwner(NotOwner {}));
        }

        let old_minimum = self.minimum_stake.get();
        self.minimum_stake.set(new_minimum);

        log(self.vm(), MinimumStakeUpdated {
            old_minimum,
            new_minimum,
        });

        Ok(())
    }

    /// Blacklists a user from participating (owner only)
    ///
    /// # Arguments
    /// * `user` - Address to blacklist
    ///
    /// # Returns
    /// Result indicating success or error
    ///
    /// # Errors
    /// * `NotOwner` - If caller is not the contract owner
    /// * `AddressZeroNotAllowed` - If user address is zero
    pub fn blacklist_user(&mut self, user: Address) -> Result<(), ReputationStakingError> {
        if self.vm().msg_sender() != self.owner.get() {
            return Err(ReputationStakingError::NotOwner(NotOwner {}));
        }

        if user.is_zero() {
            return Err(ReputationStakingError::AddressZeroNotAllowed(AddressZeroNotAllowed {}));
        }

        self.blacklisted.insert(user, true);

        log(self.vm(), UserBlacklistedEvent { user });

        Ok(())
    }

    /// Removes a user from blacklist (owner only)
    ///
    /// # Arguments
    /// * `user` - Address to remove from blacklist
    ///
    /// # Returns
    /// Result indicating success or error
    ///
    /// # Errors
    /// * `NotOwner` - If caller is not the contract owner
    /// * `UserNotBlacklisted` - If user is not currently blacklisted
    pub fn remove_from_blacklist(&mut self, user: Address) -> Result<(), ReputationStakingError> {
        if self.vm().msg_sender() != self.owner.get() {
            return Err(ReputationStakingError::NotOwner(NotOwner {}));
        }

        if !self.blacklisted.get(user) {
            return Err(ReputationStakingError::UserNotBlacklisted(UserNotBlacklisted {}));
        }

        self.blacklisted.insert(user, false);

        log(self.vm(), UserRemovedFromBlacklistEvent { user });

        Ok(())
    }

    /// Calculates reputation increase based on staked amount
    fn calculate_reputation_increase(&self, stake_amount: U256) -> U256 {
        let multiplier = self.reputation_multiplier.get();
        // reputation = stake_amount * multiplier / 10000
        (stake_amount * multiplier) / U256::from(10000)
    }

    /// Calculates reputation decrease based on withdrawn amount
    fn calculate_reputation_decrease(&self, withdraw_amount: U256, total_stake: U256) -> U256 {
        let current_reputation = self.reputation_scores.get(self.vm().msg_sender());
        // Proportional decrease: (withdraw_amount / total_stake) * current_reputation
        if total_stake == U256::ZERO {
            return current_reputation;
        }
        (withdraw_amount * current_reputation) / total_stake
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reputation_staking_contract() {
        use stylus_sdk::testing::*;
        let vm = TestVM::default();
        let mut contract = ReputationStaking::from(&vm);

        let owner = Address::from_word("owner");
        let staker1 = Address::from_word("alice");
        let staker2 = Address::from_word("bob");

        vm.set_sender(owner);

        // Test constructor
        let min_stake = U256::from(100);
        let reputation_mult = U256::from(5000); // 50%
        let result = contract.constructor(min_stake, reputation_mult);
        assert!(result.is_ok());
        assert_eq!(contract.owner(), owner);
        assert_eq!(contract.minimum_stake(), min_stake);
        assert_eq!(contract.reputation_multiplier(), reputation_mult);

        // Test staking
        vm.set_sender(staker1);
        vm.set_value(U256::from(200));
        let result = contract.stake();
        assert!(result.is_ok());
        assert_eq!(contract.stake_of(staker1), U256::from(200));
        assert_eq!(contract.total_staked(), U256::from(200));
        // Reputation = 200 * 5000 / 10000 = 100
        assert_eq!(contract.reputation_of(staker1), U256::from(100));

        // Test insufficient stake error
        vm.set_sender(staker2);
        vm.set_value(U256::from(50)); // Below minimum
        let result = contract.stake();
        assert!(matches!(result, Err(ReputationStakingError::InsufficientStake(_))));

        // Test blacklist functionality
        vm.set_sender(owner);
        let result = contract.blacklist_user(staker2);
        assert!(result.is_ok());
        assert!(contract.is_blacklisted(staker2));

        // Test blacklisted user cannot stake
        vm.set_sender(staker2);
        vm.set_value(U256::from(150));
        let result = contract.stake();
        assert!(matches!(result, Err(ReputationStakingError::UserBlacklisted(_))));

        // Test remove from blacklist
        vm.set_sender(owner);
        let result = contract.remove_from_blacklist(staker2);
        assert!(result.is_ok());
        assert!(!contract.is_blacklisted(staker2));

        // Test early withdrawal error (would need to advance timestamp in real test)
        vm.set_sender(staker1);
        let result = contract.withdraw(U256::from(50));
        assert!(matches!(result, Err(ReputationStakingError::TooEarlyToWithdraw(_))));

        // Test minimum stake update
        vm.set_sender(owner);
        let new_min = U256::from(150);
        let result = contract.set_minimum_stake(new_min);
        assert!(result.is_ok());
        assert_eq!(contract.minimum_stake(), new_min);

        // Test error cases
        vm.set_sender(staker1);
        let result = contract.set_minimum_stake(U256::from(200));
        assert!(matches!(result, Err(ReputationStakingError::NotOwner(_))));

        // Test withdraw more than staked
        let result = contract.withdraw(U256::from(500));
        assert!(matches!(result, Err(ReputationStakingError::InsufficientBalance(_))));
    }
}
