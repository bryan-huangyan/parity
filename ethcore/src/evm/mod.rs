// Copyright 2015-2017 Parity Technologies (UK) Ltd.
// This file is part of Parity.

// Parity is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Parity is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Parity.  If not, see <http://www.gnu.org/licenses/>.

//! Ethereum virtual machine.

pub mod ext;
pub mod evm;
pub mod interpreter;
#[macro_use]
pub mod factory;
pub mod schedule;
pub mod wasm;

mod vmtype;
mod instructions;
#[cfg(feature = "jit" )]
mod jit;

#[cfg(test)]
mod tests;
#[cfg(all(feature="benches", test))]
mod benches;

pub use self::evm::{Evm, Error, Finalize, FinalizationResult, GasLeft, Result, CostType, ReturnData};
pub use self::ext::{Ext, ContractCreateResult, MessageCallResult, CreateContractAddress};
pub use self::instructions::{InstructionInfo, INSTRUCTIONS, push_bytes};
pub use self::vmtype::VMType;
pub use self::factory::Factory;
pub use self::schedule::{Schedule, CleanDustMode};
pub use types::executed::CallType;
