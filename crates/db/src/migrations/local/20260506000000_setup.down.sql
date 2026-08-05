-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

drop table if exists sensor_measurements;

drop trigger if exists actuator_rules_set_updated_at on actuator_rules;
drop trigger if exists actuators_set_updated_at on actuators;
drop trigger if exists sensors_set_updated_at on sensors;
drop trigger if exists station_knots_set_updated_at on station_knots;
drop trigger if exists station_hubs_set_updated_at on station_hubs;
drop trigger if exists users_set_updated_at on users;
drop function if exists set_updated_at();

drop table if exists actuator_rules;
drop table if exists actuators;
drop table if exists sensors;
drop table if exists station_knots;
drop table if exists station_hubs;
drop table if exists users;

drop type if exists actuator_status;
drop type if exists actuator_backend;
drop type if exists actuator_protocol;
drop type if exists actuator_kind;
drop type if exists sensor_status;
drop type if exists sensor_driver;
drop type if exists sensor_kind;
drop type if exists sensor_protocol;
drop type if exists station_knot_status;
drop type if exists station_knot_role;
