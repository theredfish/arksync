drop trigger if exists station_hubs_create_local_hub_knot on station_hubs;
drop function if exists create_local_hub_knot();

drop trigger if exists discovered_hardware_set_updated_at on discovered_hardware;
drop trigger if exists actuators_set_updated_at on actuators;
drop trigger if exists sensors_set_updated_at on sensors;
drop trigger if exists station_knots_set_updated_at on station_knots;
drop trigger if exists station_hubs_set_updated_at on station_hubs;
drop trigger if exists users_set_updated_at on users;
drop function if exists set_updated_at();

drop table if exists discovered_hardware;
drop table if exists actuators;
drop table if exists sensors;
drop table if exists station_knots;
drop table if exists station_hubs;
drop table if exists users;

drop type if exists discovered_hardware_kind;
drop type if exists actuator_status;
drop type if exists actuator_kind;
drop type if exists sensor_status;
drop type if exists sensor_driver;
drop type if exists sensor_kind;
drop type if exists hw_transport;
drop type if exists station_knot_status;
drop type if exists station_knot_role;
drop type if exists station_hub_status;
drop type if exists hw_architecture;
drop type if exists station_knot_hw_kind;
drop type if exists station_hub_hw_kind;
