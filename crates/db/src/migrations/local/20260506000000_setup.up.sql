create type station_hub_hw_kind as enum (
    'RaspberryPi4B',
    'Pc'
);

create type station_knot_hw_kind as enum (
    'RaspberryPi4B',
    'Esp32',
    'Pc'
);

create type hw_architecture as enum (
    'Arm32',
    'Arm64',
    'X86',
    'X86_64',
    'Xtensa'
);

create type station_hub_status as enum (
    'Active',
    'Disabled',
    'Unreachable'
);

create type station_knot_role as enum (
    'LocalHub',
    'RemoteKnot'
);

create type station_knot_status as enum (
    'Active',
    'Disabled',
    'Unreachable'
);

create type hw_transport as enum (
    'Uart',
    'I2c',
    'Gpio'
);

create type sensor_kind as enum (
    'Temperature',
    'Ph',
    'Ec',
    'Humidity',
    'Co2',
    'Custom'
);

create type sensor_driver as enum (
    'EzoRtd'
);

create type sensor_status as enum (
    'Active',
    'Degraded',
    'Initializing',
    'Unplugged',
    'Unreachable'
);

create type actuator_kind as enum (
    'Relay',
    'Pump',
    'Valve',
    'Fan',
    'Light',
    'Custom'
);

create type actuator_status as enum (
    'Active',
    'Disabled',
    'Unreachable'
);

create type discovered_hardware_kind as enum (
    'Sensor',
    'Actuator',
    'StationKnot',
    'Unknown'
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
    hardware_kind station_hub_hw_kind not null,
    architecture hw_architecture not null,
    status station_hub_status not null default 'Active',
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
    hardware_kind station_knot_hw_kind not null,
    architecture hw_architecture not null,
    status station_knot_status not null default 'Active',
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    deleted_at timestamptz
);

create unique index station_knots_hardware_uid_unique
on station_knots (hardware_uid)
where deleted_at is null;

create unique index station_knots_one_local_hub_per_station_hub
on station_knots (station_hub_id)
where role = 'LocalHub' and deleted_at is null;

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
    transport hw_transport not null,
    connection jsonb not null default '{}'::jsonb,
    firmware double precision,
    status sensor_status not null default 'Initializing',
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
    kind actuator_kind not null,
    transport hw_transport not null,
    connection jsonb not null default '{}'::jsonb,
    status actuator_status not null default 'Active',
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

create table discovered_hardware (
    id uuid primary key default gen_random_uuid(),
    station_hub_id uuid not null references station_hubs(id),
    hardware_uid text,
    kind discovered_hardware_kind not null default 'Unknown',
    transport hw_transport not null,
    connection jsonb not null default '{}'::jsonb,
    metadata jsonb not null default '{}'::jsonb,
    first_seen_at timestamptz not null default now(),
    last_seen_at timestamptz not null default now(),
    created_at timestamptz not null default now(),
    updated_at timestamptz not null default now(),
    deleted_at timestamptz
);

create index discovered_hardware_station_hub_id_idx
on discovered_hardware (station_hub_id)
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

create trigger discovered_hardware_set_updated_at
before update on discovered_hardware
for each row
execute function set_updated_at();

create function create_local_hub_knot()
returns trigger
language plpgsql
as $$
begin
    insert into station_knots (
        station_hub_id,
        name,
        hardware_uid,
        role,
        hardware_kind,
        architecture,
        status
    )
    values (
        new.id,
        new.name,
        new.hardware_uid,
        'LocalHub',
        new.hardware_kind::text::station_knot_hw_kind,
        new.architecture,
        'Active'
    );

    return new;
end;
$$;

create trigger station_hubs_create_local_hub_knot
after insert on station_hubs
for each row
execute function create_local_hub_knot();
