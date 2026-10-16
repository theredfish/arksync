-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

-- Restoring the old global EventId key fails safely if scoped IDs now collide.
alter table knot_message_receipts
    drop constraint knot_message_receipts_owner_fkey,
    drop constraint knot_message_receipts_pkey,
    drop column hub_id,
    add primary key (event_id);
alter table station_knots drop constraint station_knots_hub_hardware_uid_key;
drop index station_knots_hardware_uid_unique;
create unique index station_knots_hardware_uid_unique
on station_knots (hardware_uid) where deleted_at is null;
