-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

create type dashboard_component_kind as enum (
    'gauge',
    'line_chart'
);

create table dashboards (
    id uuid primary key default gen_random_uuid(),
    name text not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    deleted_at timestamptz
);

create unique index dashboards_name_unique
on dashboards (name)
where deleted_at is null;

create table dashboard_components (
    id uuid primary key default gen_random_uuid(),
    dashboard_id uuid not null references dashboards(id),
    sensor_id uuid references sensors(id),
    component_kind dashboard_component_kind not null,
    title text not null,
    refresh_interval_ms integer not null check (refresh_interval_ms > 0),
    config jsonb not null default '{}'::jsonb,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    deleted_at timestamptz
);

create index dashboard_components_dashboard_id_idx
on dashboard_components (dashboard_id)
where deleted_at is null;

create index dashboard_components_sensor_id_idx
on dashboard_components (sensor_id)
where sensor_id is not null and deleted_at is null;

create trigger dashboards_set_updated_at
before update on dashboards
for each row
execute function set_updated_at();

create trigger dashboard_components_set_updated_at
before update on dashboard_components
for each row
execute function set_updated_at();
