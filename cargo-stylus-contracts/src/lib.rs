#![cfg_attr(not(feature = "export-abi"), no_main)]
extern crate alloc;
//0x16f7b54cb4002b5ca98a07ee44d81802e1009977
use stylus_sdk::{
    alloy_primitives::{U256, Address},
    alloy_sol_types::sol,
    prelude::*,
    storage::{StorageVec, StorageString},
    evm,  // Add evm module
    msg,  // Add msg module for msg::sender()
    block, // Add block module for block::timestamp()
};
use alloc::string::String;

// Define events and errors using sol! macro
sol! {
    event VoteCast(string tag_id, uint256 button_number, uint256 timestamp);
    event WinnerDeclared(uint256 winning_button, uint256 votes);
    event OwnershipTransferred(address indexed previous_owner, address indexed new_owner);
    
    // Define error types
    error AlreadyVoted(string message);
    error NoVotes(string message);
    error InvalidIndex(string message);
    error NotOwner(string message);
    error ReentrancyGuard(string message);
}

// Storage structs
sol_storage! {
    #[entrypoint]
    pub struct RFIDVoting {
        address owner;
        StorageVec<VoteData> votes;
        mapping(string => bool) has_voted;
        mapping(uint256 => uint256) button_votes;
        bool locked;
    }
    
    pub struct VoteData {
        StorageString tag_id;
        uint256 button_number;
        uint256 timestamp;
    }
}

// Error enum that wraps sol! errors
#[derive(SolidityError)]
pub enum RFIDVotingError {
    AlreadyVoted(AlreadyVoted),
    NoVotes(NoVotes),
    InvalidIndex(InvalidIndex),
    NotOwner(NotOwner),
    ReentrancyGuard(ReentrancyGuard),
}

#[public]
impl RFIDVoting {
    // Constructor - call this after deployment
    pub fn initialize(&mut self) -> Result<(), RFIDVotingError> {
        let caller = msg::sender();
        self.owner.set(caller);
        self.locked.set(false);
        Ok(())
    }

    // Cast vote with reentrancy guard
    pub fn cast_vote(&mut self, tag_id: String, button_number: U256) -> Result<(), RFIDVotingError> {
        // Reentrancy guard
        if self.locked.get() {
            return Err(RFIDVotingError::ReentrancyGuard(ReentrancyGuard {
                message: String::from("Reentrant call"),
            }));
        }
        self.locked.set(true);

        // Check if already voted
        if self.has_voted.get(tag_id.clone()) {
            self.locked.set(false);
            return Err(RFIDVotingError::AlreadyVoted(AlreadyVoted {
                message: String::from("This tag has already voted."),
            }));
        }

        // Get current timestamp
        let timestamp = U256::from(block::timestamp());

        // Create new vote
        let mut new_vote = self.votes.grow();
        new_vote.tag_id.set_str(&tag_id);
        new_vote.button_number.set(button_number);
        new_vote.timestamp.set(timestamp);

        // Mark as voted
        self.has_voted.insert(tag_id.clone(), true);

        // Increment button votes
        let current_count = self.button_votes.get(button_number);
        self.button_votes.insert(button_number, current_count + U256::from(1));

        // Emit event
        evm::log(VoteCast {
            tag_id,
            button_number,
            timestamp,
        });

        self.locked.set(false);
        Ok(())
    }

    // Get total vote count
    pub fn get_vote_count(&self) -> U256 {
        U256::from(self.votes.len())
    }

    // Get vote by index
    pub fn get_vote(&self, index: U256) -> Result<(String, U256, U256), RFIDVotingError> {
        let idx = index.to::<usize>();
        
        if idx >= self.votes.len() {
            return Err(RFIDVotingError::InvalidIndex(InvalidIndex {
                message: String::from("Invalid vote index."),
            }));
        }

        let vote = self.votes.get(idx).unwrap();
        let tag_id = vote.tag_id.get_string();
        let button_number = vote.button_number.get();
        let timestamp = vote.timestamp.get();

        Ok((tag_id, button_number, timestamp))
    }

    // Pick winner - view function (no owner check needed)
    pub fn pick_winner(&self) -> Result<(U256, U256), RFIDVotingError> {
        // Check if votes exist
        if self.votes.len() == 0 {
            return Err(RFIDVotingError::NoVotes(NoVotes {
                message: String::from("No votes have been cast yet."),
            }));
        }

        let mut max_votes = U256::ZERO;
        let mut winning_btn = U256::ZERO;

        // Find winner
        for i in 0..self.votes.len() {
            let vote = self.votes.get(i).unwrap();
            let btn = vote.button_number.get();
            let vote_count = self.button_votes.get(btn);

            if vote_count > max_votes {
                max_votes = vote_count;
                winning_btn = btn;
            }
        }

        Ok((winning_btn, max_votes))
    }

    // Reset vote for a tag (only owner)
    pub fn reset_vote(&mut self, tag_id: String) -> Result<(), RFIDVotingError> {
        if msg::sender() != self.owner.get() {
            return Err(RFIDVotingError::NotOwner(NotOwner {
                message: String::from("Caller is not the owner"),
            }));
        }

        self.has_voted.insert(tag_id, false);
        Ok(())
    }

    // Get owner
    pub fn owner(&self) -> Address {
        self.owner.get()
    }

    // Transfer ownership
    pub fn transfer_ownership(&mut self, new_owner: Address) -> Result<(), RFIDVotingError> {
        if msg::sender() != self.owner.get() {
            return Err(RFIDVotingError::NotOwner(NotOwner {
                message: String::from("Caller is not the owner"),
            }));
        }

        let old_owner = self.owner.get();
        self.owner.set(new_owner);

        evm::log(OwnershipTransferred {
            previous_owner: old_owner,
            new_owner,
        });

        Ok(())
    }

    // Get button vote count
    pub fn get_button_votes(&self, button_number: U256) -> U256 {
        self.button_votes.get(button_number)
    }

    // Check if tag has voted
    pub fn check_has_voted(&self, tag_id: String) -> bool {
        self.has_voted.get(tag_id)
    }
}