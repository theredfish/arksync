-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

-- Removing a Knot must not free its hardware identity for another Hub.
drop index station_knots_hardware_uid_unique;
create unique index station_knots_hardware_uid_unique on station_knots (hardware_uid);
alter table station_knots
add constraint station_knots_hub_hardware_uid_key unique (station_hub_id, hardware_uid);

alter table knot_message_receipts add column hub_id uuid;
update knot_message_receipts r
set hub_id = k.station_hub_id
from station_knots k where k.hardware_uid = r.source_hardware_uid;
alter table knot_message_receipts
    alter column hub_id set not null,
    drop constraint knot_message_receipts_pkey,
    add primary key (hub_id, source_hardware_uid, event_id),
    add constraint knot_message_receipts_owner_fkey
        foreign key (hub_id, source_hardware_uid)
        references station_knots (station_hub_id, hardware_uid);
