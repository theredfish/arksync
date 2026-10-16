-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

-- Rename existing objects in place, preserving rows, references and enum values.
alter table station_hubs rename to hubs;
alter table station_knots rename to knots;
alter table knots rename column station_hub_id to hub_id;
alter table sensors rename column station_knot_id to knot_id;
alter table actuators rename column station_knot_id to knot_id;
alter table knot_config_deliveries rename column station_knot_id to knot_id;

alter type station_knot_role rename to knot_role;
alter type station_knot_status rename to knot_status;

alter table hubs rename constraint station_hubs_pkey to hubs_pkey;
alter table hubs rename constraint station_hubs_user_id_fkey to hubs_user_id_fkey;
alter table knots rename constraint station_knots_pkey to knots_pkey;
alter table knots rename constraint station_knots_station_hub_id_fkey to knots_hub_id_fkey;
alter table knots rename constraint station_knots_config_version_check to knots_config_version_check;
alter table knots rename constraint station_knots_hub_hardware_uid_key to knots_hub_hardware_uid_key;
alter table sensors rename constraint sensors_station_knot_id_fkey to sensors_knot_id_fkey;
alter table actuators rename constraint actuators_station_knot_id_fkey to actuators_knot_id_fkey;
alter table knot_config_deliveries rename constraint knot_config_deliveries_station_knot_id_fkey to knot_config_deliveries_knot_id_fkey;

alter index station_hubs_hardware_uid_unique rename to hubs_hardware_uid_unique;
alter index station_hubs_user_id_idx rename to hubs_user_id_idx;
alter index station_knots_hardware_uid_unique rename to knots_hardware_uid_unique;
alter index station_knots_one_local_hub_per_station_hub rename to knots_one_local_knot_per_hub;
alter index station_knots_station_hub_id_idx rename to knots_hub_id_idx;
alter index sensors_station_knot_device_uid_unique rename to sensors_knot_device_uid_unique;
alter index sensors_station_knot_id_idx rename to sensors_knot_id_idx;
alter index actuators_station_knot_device_uid_unique rename to actuators_knot_device_uid_unique;
alter index actuators_station_knot_id_idx rename to actuators_knot_id_idx;
alter index knot_config_deliveries_station_knot_id_idx rename to knot_config_deliveries_knot_id_idx;

alter trigger station_hubs_set_updated_at on hubs rename to hubs_set_updated_at;
alter trigger station_knots_set_updated_at on knots rename to knots_set_updated_at;
