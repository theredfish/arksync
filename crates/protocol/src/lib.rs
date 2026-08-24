// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

//! Runtime-independent wire contracts shared by ArkSync actors.
//!
//! The types in this crate are protocol DTOs. Runtime and domain crates map
//! their internal models explicitly so internal refactors do not silently
//! change the Postcard wire representation.

#![no_std]

extern crate alloc;

mod actor;
mod frame;
pub mod knot;

pub use actor::*;
pub use frame::*;
