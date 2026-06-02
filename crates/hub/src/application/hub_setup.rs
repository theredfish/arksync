// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::config::CONFIG;
use crate::infrastructure::store::{
    upsert_station_hub, upsert_station_knot, upsert_system_user, HubRecord, KnotRecord,
    SystemUserRecord,
};

pub async fn setup_local_station(executor: &sqlx::PgPool) -> Result<(), sqlx::Error> {
    let user = SystemUserRecord {
        id: CONFIG.local_system_user_id,
        username: CONFIG.local_system_username.clone(),
        password: CONFIG.local_system_password.clone(),
    };
    let hub = HubRecord {
        id: CONFIG.local_hub_id,
        user_id: CONFIG.local_system_user_id,
        name: CONFIG.local_hub_name.clone(),
        hardware_uid: CONFIG.local_hub_hardware_uid.clone(),
    };
    let knot = KnotRecord {
        id: CONFIG.local_knot_id,
        hub_id: CONFIG.local_hub_id,
        name: CONFIG.local_knot_name.clone(),
        hardware_uid: CONFIG.local_knot_hardware_uid.clone(),
    };

    upsert_system_user(executor, &user).await?;
    upsert_station_hub(executor, &hub).await?;
    upsert_station_knot(executor, &knot).await
}
