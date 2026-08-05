-- This Source Code Form is subject to the terms of the Mozilla Public
-- License, v. 2.0. If a copy of the MPL was not distributed with this
-- file, You can obtain one at https://mozilla.org/MPL/2.0/.

drop trigger if exists dashboard_components_set_updated_at on dashboard_components;
drop trigger if exists dashboards_set_updated_at on dashboards;

drop table if exists dashboard_components;
drop table if exists dashboards;

drop type if exists dashboard_component_kind;
