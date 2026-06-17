// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_utils::uuid::Uuid;
use sqlx::FromRow;

#[derive(Clone, Debug, FromRow)]
pub struct KnotRecord {
    pub id: Uuid,
    pub hub_id: Uuid,
    pub name: String,
    pub hardware_uid: String,
    pub role: String,
    pub status: String,
}
