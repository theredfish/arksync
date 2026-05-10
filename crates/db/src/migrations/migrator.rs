// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use std::future::Future;

pub trait Migrator {
    fn run() -> impl Future<Output = Result<(), sqlx::migrate::MigrateError>> + Send;
}
