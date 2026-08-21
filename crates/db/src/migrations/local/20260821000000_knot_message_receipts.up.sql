-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

alter table station_knots
add column config_version bigint not null default 1 check (config_version > 0),
add column applied_config_version bigint,
add column config_status text not null default 'pending',
add column config_error text;

create table knot_message_receipts (
    event_id uuid primary key,
    source_hardware_uid text not null,
    message_kind text not null,
    processed_at timestamptz not null default now()
);

create index knot_message_receipts_source_hardware_uid_idx
on knot_message_receipts (source_hardware_uid);
