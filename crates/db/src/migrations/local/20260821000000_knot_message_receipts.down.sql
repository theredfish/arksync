-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

drop table if exists knot_message_receipts;

alter table station_knots
drop column if exists config_error,
drop column if exists config_status,
drop column if exists applied_config_version,
drop column if exists config_version;
