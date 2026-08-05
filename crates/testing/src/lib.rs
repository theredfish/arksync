// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

mod postgres;

pub use arksync_macros::test;
pub use postgres::{test_succeeded, PgPool, TestDatabase};

#[doc(hidden)]
pub use tokio;
