-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

create table sensor_measurements (
    id uuid not null default gen_random_uuid(),
    event_id uuid not null,
    source_parent_hub_id uuid not null,
    source_knot_id uuid not null,
    sensor_id uuid not null references sensors(id),
    sensor_kind sensor_kind not null,
    unit text not null,
    value double precision not null,
    measured_at timestamptz not null,
    received_at timestamptz not null,
    created_at timestamptz not null default now()
) partition by range (measured_at);

create index sensor_measurements_sensor_id_measured_at_idx
on sensor_measurements (sensor_id, measured_at desc);

select public.create_parent(
    p_parent_table := 'public.sensor_measurements',
    p_control := 'measured_at',
    p_interval := '1 day',
    p_type := 'range',
    p_premake := 7,
    p_start_partition := date_trunc('day', now())::text
);
