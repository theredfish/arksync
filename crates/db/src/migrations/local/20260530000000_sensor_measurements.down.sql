-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

delete from public.part_config
where parent_table = 'public.sensor_measurements';

drop table if exists sensor_measurements cascade;
