// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use core::cell::RefCell;
use critical_section::Mutex;
use rand::rngs::SmallRng;
use rand::SeedableRng;

static DEVICE_UID_RNG: Mutex<RefCell<Option<SmallRng>>> = Mutex::new(RefCell::new(None));

#[cfg(feature = "std")]
pub fn init_from_os_rng() {
    use rand::rngs::OsRng;
    use rand::TryRngCore;

    let mut seed = [0_u8; 32];

    OsRng
        .try_fill_bytes(&mut seed)
        .expect("OS RNG should seed ArkSync device UID RNG");

    init(seed);
}

pub fn init(seed: [u8; 32]) {
    critical_section::with(|critical_section| {
        DEVICE_UID_RNG
            .borrow_ref_mut(critical_section)
            .replace(SmallRng::from_seed(seed));
    });
}

pub(crate) fn with<T>(f: impl FnOnce(&mut SmallRng) -> T) -> T {
    critical_section::with(|critical_section| {
        let mut rng = DEVICE_UID_RNG.borrow_ref_mut(critical_section);
        let rng = rng
            .as_mut()
            .expect("ArkSync device UID RNG must be initialized before DeviceUid::new");

        f(rng)
    })
}
