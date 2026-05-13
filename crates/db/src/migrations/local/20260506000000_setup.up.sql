create type station_knot_role as enum (
    'local_hub',
    'remote_knot'
);

create type station_knot_status as enum (
    'awake',
    'sleeping',
    'unreachable'
);

create type sensor_protocol as enum (
    'uart',
    'i2c'
);

create type sensor_kind as enum (
    'temperature',
    'ph',
    'ec',
    'humidity',
    'co2',
    'custom'
);

create type sensor_driver as enum (
    'atlas_scientific_ezo'
);

create type sensor_status as enum (
    'active',
    'degraded',
    'initializing',
    'unplugged',
    'unreachable'
);

create type actuator_status as enum (
    'active',
    'disabled',
    'unreachable'
);

create table users (
    id uuid primary key default gen_random_uuid(),
    username text not null,
    password text not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    deleted_at timestamptz
);

create unique index users_username_unique
on users (username)
where deleted_at is null;

create table station_hubs (
    id uuid primary key default gen_random_uuid(),
    user_id uuid not null references users(id),
    name text not null,
    hardware_uid text not null,
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    deleted_at timestamptz
);

create unique index station_hubs_hardware_uid_unique
on station_hubs (hardware_uid)
where deleted_at is null;

create index station_hubs_user_id_idx
on station_hubs (user_id)
where deleted_at is null;

create table station_knots (
    id uuid primary key default gen_random_uuid(),
    station_hub_id uuid not null references station_hubs(id),
    name text not null,
    hardware_uid text not null,
    role station_knot_role not null,
    status station_knot_status not null default 'awake',
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    deleted_at timestamptz
);

create unique index station_knots_hardware_uid_unique
on station_knots (hardware_uid)
where deleted_at is null;

create unique index station_knots_one_local_hub_per_station_hub
on station_knots (station_hub_id)
where role = 'local_hub' and deleted_at is null;

create index station_knots_station_hub_id_idx
on station_knots (station_hub_id)
where deleted_at is null;

create table sensors (
    id uuid primary key default gen_random_uuid(),
    station_knot_id uuid not null references station_knots(id),
    hardware_uid text,
    name text not null,
    kind sensor_kind not null,
    driver sensor_driver not null,
    protocol sensor_protocol not null,
    connection jsonb not null default '{}'::jsonb,
    firmware double precision,
    status sensor_status not null default 'initializing',
    state_reason text not null default 'plugged',
    state_reason_details jsonb not null default '{}'::jsonb,
    state_since timestamptz not null default now(),
    last_activity_at timestamptz not null default now(),
    consecutive_failures integer not null default 0 check (consecutive_failures >= 0),
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    deleted_at timestamptz
);

create unique index sensors_hardware_uid_unique
on sensors (hardware_uid)
where hardware_uid is not null and deleted_at is null;

create index sensors_station_knot_id_idx
on sensors (station_knot_id)
where deleted_at is null;

create table actuators (
    id uuid primary key default gen_random_uuid(),
    station_knot_id uuid not null references station_knots(id),
    hardware_uid text,
    name text not null,
    kind text not null,
    protocol text not null,
    connection jsonb not null default '{}'::jsonb,
    status actuator_status not null default 'active',
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    deleted_at timestamptz
);

create unique index actuators_hardware_uid_unique
on actuators (hardware_uid)
where hardware_uid is not null and deleted_at is null;

create index actuators_station_knot_id_idx
on actuators (station_knot_id)
where deleted_at is null;

create function set_updated_at()
returns trigger
language plpgsql
as $$
begin
    new.updated_at = now();
    return new;
end;
$$;

create trigger users_set_updated_at
before update on users
for each row
execute function set_updated_at();

create trigger station_hubs_set_updated_at
before update on station_hubs
for each row
execute function set_updated_at();

create trigger station_knots_set_updated_at
before update on station_knots
for each row
execute function set_updated_at();

create trigger sensors_set_updated_at
before update on sensors
for each row
execute function set_updated_at();

create trigger actuators_set_updated_at
before update on actuators
for each row
execute function set_updated_at();

create function register_local_hub_as_knot()
returns trigger
language plpgsql
as $$
begin
    insert into station_knots (
        station_hub_id,
        name,
        hardware_uid,
        role,
        status
    )
    values (
        new.id,
        new.name,
        new.hardware_uid,
        'local_hub',
        'active'
    );

    return new;
end;
$$;

create trigger trigger_register_local_hub_as_knot
after insert on station_hubs
for each row
execute function register_local_hub_as_knot();
