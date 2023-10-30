use crate::{
    unit::{mount, Unit, UnitCommon, UnitImpl},
    util::loader::load_units_from_dir,
};

use super::{
    unit::utils::{update_unit, update_units},
    Actors,
};

async fn wait() {
    use std::time::Duration;
    use tokio::time::sleep;
    // println!("waiting start");
    sleep(Duration::from_secs_f64(0.1)).await;
    // println!("waiting end");
}

#[test]
fn test_basic() {
    use crate::{actor::unit::utils::stop_unit, unit::UnitId, util::loader::load_units_from_dir};

    use super::{
        unit::utils::{start_unit, update_units},
        Actors,
    };

    let result = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let actors = Actors::new();
            update_units(&actors.store, load_units_from_dir("./units").await).await;

            // start unit with its dep
            start_unit(&actors.store, UnitId::from("t0.service")).await;
            wait().await;

            // stop the active unit
            stop_unit(&actors.store, UnitId::from("t0.service")).await;
            wait().await;

            // start a stopped unit, with its dep started
            start_unit(&actors.store, UnitId::from("t0.service")).await;
            wait().await;
            stop_unit(&actors.store, UnitId::from("t0.service")).await;
            wait().await;

            // start the stopping unit
            start_unit(&actors.store, UnitId::from("t0.service")).await;
            wait().await;
            stop_unit(&actors.store, UnitId::from("t0.service")).await;
            start_unit(&actors.store, UnitId::from("t0.service")).await; // bug: not started
            wait().await;
            stop_unit(&actors.store, UnitId::from("t0.service")).await;
            wait().await;

            // stop a starting unit
            start_unit(&actors.store, UnitId::from("t0.service")).await;
            stop_unit(&actors.store, UnitId::from("t0.service")).await;
            wait().await;

            stop_unit(&actors.store, UnitId::from("t1.service")).await;
            wait().await;
        });
}

#[test]
fn test_conflict() {
    use crate::{actor::unit::utils::stop_unit, unit::UnitId, util::loader::load_units_from_dir};

    use super::{
        unit::utils::{start_unit, update_units},
        Actors,
    };

    let result = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let actors = Actors::new();
            update_units(&actors.store, load_units_from_dir("./units").await).await;

            start_unit(&actors.store, UnitId::from("t1.service")).await;
            wait().await;
            start_unit(&actors.store, UnitId::from("conflict-with-t1.service")).await;
            wait().await;
            // t1 should stop

            start_unit(&actors.store, UnitId::from("t0.service")).await;
            wait().await;
            // conflict-with-t1 should stop

            start_unit(&actors.store, UnitId::from("conflict-with-t1.service")).await;
            wait().await;

            stop_unit(&actors.store, UnitId::from("conflict-with-t1.service")).await;
            wait().await;
        });
}

fn test_mount() {
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            let actors = Actors::new();
            update_units(&actors.store, load_units_from_dir("./units").await).await;
        });
}
