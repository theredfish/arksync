// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use arksync_utils::uuid::Uuid;

#[derive(Clone, Debug)]
pub struct SystemUserRecord {
    pub id: Uuid,
    pub username: String,
    pub password: String,
}

#[derive(Clone, Debug)]
pub struct HubRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub name: String,
    pub hardware_uid: String,
}
