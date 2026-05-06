// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

#[test]
fn connects_and_runs_migrations() {
    let runtime = tokio::runtime::Runtime::new().expect("runtime");

    runtime.block_on(async {
        let selected_value: i32 = sqlx::query_scalar("select 1")
            .fetch_one(arksync_db::pool())
            .await
            .expect("connect to postgres");

        assert_eq!(selected_value, 1);

        arksync_db::setup().await.expect("setup postgres");
        arksync_db::reset_public_schema()
            .await
            .expect("reset public schema");
        arksync_db::setup()
            .await
            .expect("setup postgres after schema reset");
        arksync_db::migrations::run().await.expect("run migrations");
    });
}
