// Copyright 2017-2019 Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! # Council Module
//!
//! The Council module provides tools to manage the council and proposals.
//!
//! - **Seats**
//! 	- [`seats::Trait`](./seats/trait.Trait.html)
//! 	- [`Call`](./seats/enum.Call.html)
//! 	- [`Module`](./seats/struct.Module.html)
//! - **Motions**
//! 	- [`motions::Trait`](./motions/trait.Trait.html)
//! 	- [`Call`](./motions/enum.Call.html)
//! 	- [`Module`](./motions/struct.Module.html)
//! - **Voting**
//! 	- [`voting::Trait`](./voting/trait.Trait.html)
//! 	- [`Call`](./voting/enum.Call.html)
//! 	- [`Module`](./voting/struct.Module.html)
//!
//! ## Overview
//!
//! The Council module provides functionality to handle:
//!
//! - The voting in and maintenance of council members.
//! - Proposing, vetoing, and passing motions.
//!
//! The Council's role is to represent passive stakeholders. The Council is an on-chain entity comprised of
//! a set of account IDs. Its primary tasks are to propose sensible referenda and thwart any uncontroversially
//! dangerous or malicious referenda.
//!
//! ### Terminology
//!
//! #### Council Proposals
//!
//! - **Council motion:** A mechanism used to enact a proposal.
//! - **Council origin:** The council (not root) that contains the council motion mechanism.
//! - **Proposal validity:** A council proposal is valid when it's unique, hasn't yet been vetoed, and
//! when the council term doesn't expire before the block number when the proposal's voting period ends.
//! - **Proposal postponement:** Councillors that abstain from voting may postpone a council proposal from
//! being approved or rejected. Postponement is equivalent to a veto, which only lasts for the cooloff period.
//! - **Cooloff period:** Period, in blocks, for which a veto is in effect.
//!
//! #### Council Proposal Voting
//!
//! - **Proposal:** A submission by a councillor. An initial vote of yay from that councillor is applied.
//! Unlike the Democracy and Treasury modules, the `Proposal` type is very generic.
//! - **Referendum:** The means of voting on a proposal.
//! - **Vote:** A vote of yay or nay from a councillor on a single proposal. Councillors may change their vote.
//! - **Veto:** A council member may veto any council proposal that exists. A vetoed proposal that's valid is set
//! aside for a cooloff period. The vetoer cannot re-veto or propose the proposal again until the veto expires.
//! - **Vote cancellation:** At the end of a given block we cancel all referenda that have been
//! elevated to the Table of Referenda whose voting period ends at that block and where the outcome of the vote
//! tally was a unanimous vote to cancel the referendum.
//! - **Voting process to elevate a proposal:** At the end of a given block we tally votes for expiring referenda.
//! Referenda that are passed (yay votes are greater than nay votes plus abstainers) are sent to the Democracy
//! module for a public referendum. If there are no nay votes (abstention is acceptable), then the proposal is
//! for immediate enactment. Otherwise, there will be a delay period. If the vote is unanimous, then the public
//! referendum will require a vote threshold of supermajority against to prevent it. Otherwise,
//! it is a simple majority vote.
//!
//! #### Council Seats
//!
//! - **Desired seats:** The number of seats on the council. Can change via governance.
//! - **Candidacy bond:** Bond required to be a candidate.
//! - **Voting bond:** Bond required to be permitted to vote. Must be held because many voting operations affect
//! storage. The bond is held to disincent abuse.
//! - **Candidate approval voting call:** Express candidate approval voting is a public call that anyone may execute
//! by signing and submitting an extrinsic. We ensure that information about the `origin` where the dispatch initiated
//! is a signed account using `ensure_signed`.
//! - **Reaping process:** Councillors may propose the removal of other, inactive councillors. If the claim is not
//! valid, the reporter will be slashed. See the [Staking module](../srml_staking/index.html) for more information
//! on slashing.
//!
//! ### Goals
//!
//! The Council module in Substrate is designed to make the following possible:
//!
//! - Create council proposals by councillors using the council motion mechanism.
//! - Validate council proposals.
//! - Tally votes of council proposals by councillors during the proposal's voting period.
//! - Veto (postpone) council proposals for a cooloff period through abstention by councillors.
//! - Elevate council proposals to start a public referendum.
//! - Execute referenda once their vote tally reaches the vote threshold level of approval.
//! - Manage candidacy, including voting, term expiration, and punishment.
//!
//! ## Interface
//!
//! ### Dispatchable Functions
//!
//! The dispatchable functions in the Council module provide the functionality that councillors need.
//! See the `Call` enums from the motions, seats, and voting modules for details on dispatchable functions.
//!
//! ### Public Functions
//!
//! The public functions provide the functionality for other modules to interact with the Council module.
//! See the `Module` structs from the motions, seats, and voting modules for details on public functions.
//!
//! ### Example
//!
//! This code snippet includes an `approve_all` public function that could be called to approve
//! the eligibility of all candidates when there are empty council seats and when the tally for
//! the next election occurs at the current or a future block number.
//!
//! ```
//! use srml_support::{decl_module, dispatch::Result};
//! use system::ensure_signed;
//! use srml_council::seats;
//!
//! pub trait Trait: seats::Trait {}
//!
//! decl_module! {
//! 	pub struct Module<T: Trait> for enum Call where origin: T::Origin {
//!
//! 		pub fn approve_all(origin) -> Result {
//! 			let _origin = ensure_signed(origin)?;
//!
//! 			// Get the current block number
//! 			let current_block_number = <system::Module<T>>::block_number();
//!
//! 			// Get the number of seats that we want the council to have
//! 			let desired = <seats::Module<T>>::desired_seats() as usize;
//!
//! 			// Get the number of seats occupied by the current council.
//! 			let occupied = <seats::Module<T>>::active_council().len();
//!
//! 			// Get the appropriate block number to schedule the next tally.
//! 			let maybe_next_tally = <seats::Module<T>>::next_tally();
//!
//! 			assert!(desired > occupied, "Unable to approve all candidates when there are no empty seats");
//!
//! 			if let Some(next_tally_block_number) = <seats::Module<T>>::next_tally() {
//! 				if current_block_number <= next_tally_block_number {
//! 					assert!(maybe_next_tally.is_some(),
//! 						"Unable to approve all candidates when the block number of the next tally has past");
//! 				}
//! 			}
//!
//! 			Ok(())
//! 		}
//! 	}
//! }
//! # fn main() { }
//! ```
//!
//! ## Genesis config
//!
//! The Council module depends on the `GenesisConfig`.
//!
//! - [Seats](./seats/struct.GenesisConfig.html)
//! - [Voting](./voting/struct.GenesisConfig.html)
//!
//! ## Related Modules
//!
//! - [Democracy](../srml_democracy/index.html)
//! - [Staking](../srml_staking/index.html)

#![cfg_attr(not(feature = "std"), no_std)]

pub mod voting;
pub mod motions;
pub mod seats;

pub use crate::seats::{Trait, Module, RawEvent, Event, VoteIndex};

#[cfg(test)]
mod tests {
	// These re-exports are here for a reason, edit with care
	pub use super::*;
	pub use runtime_io::with_externalities;
	use srml_support::{impl_outer_origin, impl_outer_event, impl_outer_dispatch};
	pub use substrate_primitives::H256;
	pub use primitives::BuildStorage;
	pub use primitives::traits::{BlakeTwo256, IdentityLookup};
	pub use primitives::testing::{Digest, DigestItem, Header};
	pub use substrate_primitives::{Blake2Hasher};
	pub use {seats, motions, voting};

	impl_outer_origin! {
		pub enum Origin for Test {
			motions
		}
	}

	impl_outer_event! {
		pub enum Event for Test {
			balances<T>, democracy<T>, seats<T>, voting<T>, motions<T>,
		}
	}

	impl_outer_dispatch! {
		pub enum Call for Test where origin: Origin {
			balances::Balances,
			democracy::Democracy,
		}
	}

	// Workaround for https://github.com/rust-lang/rust/issues/26925. Remove when sorted.
	#[derive(Clone, Eq, PartialEq, Debug)]
	pub struct Test;
	impl system::Trait for Test {
		type Origin = Origin;
		type Index = u64;
		type BlockNumber = u64;
		type Hash = H256;
		type Hashing = BlakeTwo256;
		type Digest = Digest;
		type AccountId = u64;
		type Lookup = IdentityLookup<Self::AccountId>;
		type Header = Header;
		type Event = Event;
		type Log = DigestItem;
	}
	impl balances::Trait for Test {
		type Balance = u64;
		type OnFreeBalanceZero = ();
		type OnNewAccount = ();
		type Event = Event;
		type TransactionPayment = ();
		type TransferPayment = ();
		type DustRemoval = ();
	}
	impl democracy::Trait for Test {
		type Currency = balances::Module<Self>;
		type Proposal = Call;
		type Event = Event;
	}
	impl seats::Trait for Test {
		type Event = Event;
		type BadPresentation = ();
		type BadReaper = ();
	}
	impl motions::Trait for Test {
		type Origin = Origin;
		type Proposal = Call;
		type Event = Event;
	}
	impl voting::Trait for Test {
		type Event = Event;
	}

	pub fn new_test_ext(with_council: bool) -> runtime_io::TestExternalities<Blake2Hasher> {
		let mut t = system::GenesisConfig::<Test>::default().build_storage().unwrap().0;
		t.extend(balances::GenesisConfig::<Test>{
			transaction_base_fee: 0,
			transaction_byte_fee: 0,
			balances: vec![(1, 10), (2, 20), (3, 30), (4, 40), (5, 50), (6, 60)],
			existential_deposit: 0,
			transfer_fee: 0,
			creation_fee: 0,
			vesting: vec![],
		}.build_storage().unwrap().0);
		t.extend(democracy::GenesisConfig::<Test>{
			launch_period: 1,
			voting_period: 3,
			minimum_deposit: 1,
			public_delay: 0,
			max_lock_periods: 6,
		}.build_storage().unwrap().0);
		t.extend(seats::GenesisConfig::<Test> {
			candidacy_bond: 9,
			voter_bond: 3,
			present_slash_per_voter: 1,
			carry_count: 2,
			inactive_grace_period: 1,
			active_council: if with_council { vec![
				(1, 10),
				(2, 10),
				(3, 10)
			] } else { vec![] },
			approval_voting_period: 4,
			presentation_duration: 2,
			desired_seats: 2,
			term_duration: 5,
		}.build_storage().unwrap().0);
		t.extend(voting::GenesisConfig::<Test> {
			cooloff_period: 2,
			voting_period: 1,
			enact_delay_period: 0,
		}.build_storage().unwrap().0);
		runtime_io::TestExternalities::new(t)
	}

	pub type System = system::Module<Test>;
	pub type Balances = balances::Module<Test>;
	pub type Democracy = democracy::Module<Test>;
	pub type Council = seats::Module<Test>;
	pub type CouncilVoting = voting::Module<Test>;
	pub type CouncilMotions = motions::Module<Test>;
}
