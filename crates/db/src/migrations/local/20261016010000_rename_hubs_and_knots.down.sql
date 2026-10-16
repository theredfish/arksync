-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

alter trigger knots_set_updated_at on knots rename to station_knots_set_updated_at;
alter trigger hubs_set_updated_at on hubs rename to station_hubs_set_updated_at;
alter index knot_config_deliveries_knot_id_idx rename to knot_config_deliveries_station_knot_id_idx;
alter index actuators_knot_id_idx rename to actuators_station_knot_id_idx;
alter index actuators_knot_device_uid_unique rename to actuators_station_knot_device_uid_unique;
alter index sensors_knot_id_idx rename to sensors_station_knot_id_idx;
alter index sensors_knot_device_uid_unique rename to sensors_station_knot_device_uid_unique;
alter index knots_hub_id_idx rename to station_knots_station_hub_id_idx;
alter index knots_one_local_knot_per_hub rename to station_knots_one_local_hub_per_station_hub;
alter index knots_hardware_uid_unique rename to station_knots_hardware_uid_unique;
alter index hubs_user_id_idx rename to station_hubs_user_id_idx;
alter index hubs_hardware_uid_unique rename to station_hubs_hardware_uid_unique;
alter table knot_config_deliveries rename constraint knot_config_deliveries_knot_id_fkey to knot_config_deliveries_station_knot_id_fkey;
alter table actuators rename constraint actuators_knot_id_fkey to actuators_station_knot_id_fkey;
alter table sensors rename constraint sensors_knot_id_fkey to sensors_station_knot_id_fkey;
alter table knots rename constraint knots_hub_hardware_uid_key to station_knots_hub_hardware_uid_key;
alter table knots rename constraint knots_config_version_check to station_knots_config_version_check;
alter table knots rename constraint knots_hub_id_fkey to station_knots_station_hub_id_fkey;
alter table knots rename constraint knots_pkey to station_knots_pkey;
alter table hubs rename constraint hubs_user_id_fkey to station_hubs_user_id_fkey;
alter table hubs rename constraint hubs_pkey to station_hubs_pkey;
alter type knot_status rename to station_knot_status;
alter type knot_role rename to station_knot_role;
alter table knot_config_deliveries rename column knot_id to station_knot_id;
alter table actuators rename column knot_id to station_knot_id;
alter table sensors rename column knot_id to station_knot_id;
alter table knots rename column hub_id to station_hub_id;
alter table knots rename to station_knots;
alter table hubs rename to station_hubs;
