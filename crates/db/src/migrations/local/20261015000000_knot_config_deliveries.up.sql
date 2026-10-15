-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

create table knot_config_deliveries (
    event_id uuid primary key,
    station_knot_id uuid not null references station_knots (id),
    config_version bigint not null check (config_version > 0),
    config jsonb not null,
    outcome text check (outcome in ('applied', 'rejected')),
    error text,
    created_at timestamptz not null default now()
);

create index knot_config_deliveries_station_knot_id_idx
on knot_config_deliveries (station_knot_id);
