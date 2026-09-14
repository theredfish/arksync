// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use crate::components::page_title::PageTitle;
#[cfg(not(feature = "demo"))]
use crate::data_sources::{load_live_sensors, LiveSensorOverview};
use crate::data_sources::{AppDataSources, DEMO_MODE};
use leptos::prelude::*;
use leptos::IntoView;
#[cfg(not(feature = "demo"))]
use wasm_bindgen_futures::spawn_local;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SensorKey {
    AirTemperature,
    WaterTemperature,
    WaterPh,
    DissolvedOxygen,
    Conductivity,
    Live(u64),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SensorTab {
    Registered,
    Detected,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConnectionState {
    Connected,
    RecentlyDisconnected,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ConnectionProtocol {
    Uart,
    I2c,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SensorItem {
    key: SensorKey,
    name: String,
    role: &'static str,
    metric: &'static str,
    unit: &'static str,
    symbol: &'static str,
    value: String,
    state: ConnectionState,
    protocol: ConnectionProtocol,
    address: String,
    connection_detail: String,
    frequency: String,
    last_activity: String,
    last_saved: String,
    first_seen: &'static str,
    device_uid: String,
    probe_serial: String,
    circuit: &'static str,
    firmware: String,
    knot: &'static str,
    registered: bool,
    calibrated: bool,
    calibration_kind: &'static str,
    calibration_points: &'static str,
    calibration_instruction: &'static str,
    last_calibration: String,
    next_calibration: String,
}

impl SensorItem {
    fn state_label(&self) -> &'static str {
        match self.state {
            ConnectionState::Connected => "Connected",
            ConnectionState::RecentlyDisconnected => "Disconnected",
        }
    }

    fn protocol_label(&self) -> &'static str {
        match self.protocol {
            ConnectionProtocol::Uart => "UART",
            ConnectionProtocol::I2c => "I2C",
        }
    }

    fn is_connected(&self) -> bool {
        self.state == ConnectionState::Connected
    }
}

#[cfg(feature = "demo")]
fn demo_sensors(
    air_temperature: Option<f32>,
    water_temperature: Option<f32>,
    water_ph: Option<f32>,
) -> Vec<SensorItem> {
    vec![
        SensorItem {
            key: SensorKey::AirTemperature,
            name: "Air temperature — Canopy".to_string(),
            role: "Air temperature",
            metric: "Temperature",
            unit: "Celsius (°C)",
            symbol: "°C",
            value: format!("{:.1}", air_temperature.unwrap_or(23.8)),
            state: ConnectionState::Connected,
            protocol: ConnectionProtocol::Uart,
            address: "/dev/ttyUSB0".to_string(),
            connection_detail: "9,600 baud".to_string(),
            frequency: "1 Hz".to_string(),
            last_activity: "2 sec ago".to_string(),
            last_saved: "8 sec ago".to_string(),
            first_seen: "May 18, 2026",
            device_uid: "sensor_rtd_8c41a7".to_string(),
            probe_serial: "PT1000-AT-0418".to_string(),
            circuit: "Atlas Scientific EZO-RTD",
            firmware: "3.7".to_string(),
            knot: "Local Raspberry Pi",
            registered: true,
            calibrated: true,
            calibration_kind: "Single point",
            calibration_points: "25.00 °C reference",
            calibration_instruction: "Place the probe in a stable reference bath, then confirm the reference temperature.",
            last_calibration: "Jul 18, 2026".to_string(),
            next_calibration: "Jul 18, 2027".to_string(),
        },
        SensorItem {
            key: SensorKey::WaterTemperature,
            name: "Water temperature — Tank 01".to_string(),
            role: "Water temperature",
            metric: "Temperature",
            unit: "Celsius (°C)",
            symbol: "°C",
            value: format!("{:.1}", water_temperature.unwrap_or(27.8)),
            state: ConnectionState::Connected,
            protocol: ConnectionProtocol::I2c,
            address: "0x66".to_string(),
            connection_detail: "Bus /dev/i2c-1".to_string(),
            frequency: "1 Hz".to_string(),
            last_activity: "2 sec ago".to_string(),
            last_saved: "7 sec ago".to_string(),
            first_seen: "May 18, 2026",
            device_uid: "sensor_rtd_a19f02".to_string(),
            probe_serial: "PT1000-WT-1103".to_string(),
            circuit: "Atlas Scientific EZO-RTD",
            firmware: "3.7".to_string(),
            knot: "Local Raspberry Pi",
            registered: true,
            calibrated: true,
            calibration_kind: "Single point",
            calibration_points: "28.00 °C reference",
            calibration_instruction: "Reconnect the probe and place it in a stable reference bath before starting calibration.",
            last_calibration: "Aug 5, 2026".to_string(),
            next_calibration: "Aug 5, 2027".to_string(),
        },
        SensorItem {
            key: SensorKey::WaterPh,
            name: "Water pH — Tank 01".to_string(),
            role: "Water chemistry",
            metric: "pH",
            unit: "pH",
            symbol: "pH",
            value: format!("{:.2}", water_ph.unwrap_or(6.78)),
            state: ConnectionState::Connected,
            protocol: ConnectionProtocol::I2c,
            address: "0x63".to_string(),
            connection_detail: "Bus /dev/i2c-1".to_string(),
            frequency: "1 Hz".to_string(),
            last_activity: "1 sec ago".to_string(),
            last_saved: "5 sec ago".to_string(),
            first_seen: "May 22, 2026",
            device_uid: "sensor_ph_73d9b2".to_string(),
            probe_serial: "ENV-40-PH-2371".to_string(),
            circuit: "Atlas Scientific EZO-pH",
            firmware: "6.1".to_string(),
            knot: "Local Raspberry Pi",
            registered: true,
            calibrated: true,
            calibration_kind: "Three point",
            calibration_points: "pH 4.00 · 7.00 · 10.00",
            calibration_instruction: "Begin with the pH 7.00 midpoint buffer, then continue with the low and high points.",
            last_calibration: "Aug 24, 2026".to_string(),
            next_calibration: "Aug 24, 2027".to_string(),
        },
        SensorItem {
            key: SensorKey::DissolvedOxygen,
            name: "Dissolved oxygen — Unnamed".to_string(),
            role: "Dissolved oxygen",
            metric: "Dissolved oxygen",
            unit: "Milligrams per litre (mg/L)",
            symbol: "O₂",
            value: "8.74".to_string(),
            state: ConnectionState::Connected,
            protocol: ConnectionProtocol::I2c,
            address: "0x61".to_string(),
            connection_detail: "Bus /dev/i2c-1".to_string(),
            frequency: "1 Hz".to_string(),
            last_activity: "Just now".to_string(),
            last_saved: "Not saved".to_string(),
            first_seen: "8 min ago",
            device_uid: "sensor_do_c04e91".to_string(),
            probe_serial: "ENV-40-DOX-0842".to_string(),
            circuit: "Atlas Scientific EZO-DO",
            firmware: "5.8".to_string(),
            knot: "Local Raspberry Pi",
            registered: false,
            calibrated: false,
            calibration_kind: "One or two point",
            calibration_points: "Air · optional zero D.O.",
            calibration_instruction: "Expose the probe to air until the reading is stable. Zero D.O. calibration is optional.",
            last_calibration: "Never".to_string(),
            next_calibration: "Not scheduled".to_string(),
        },
        SensorItem {
            key: SensorKey::Conductivity,
            name: "Conductivity — Nutrient reservoir".to_string(),
            role: "Water chemistry",
            metric: "Conductivity",
            unit: "Microsiemens per centimetre (µS/cm)",
            symbol: "µS/cm",
            value: "2,980".to_string(),
            state: ConnectionState::RecentlyDisconnected,
            protocol: ConnectionProtocol::I2c,
            address: "0x64".to_string(),
            connection_detail: "Bus /dev/i2c-1".to_string(),
            frequency: "1 Hz".to_string(),
            last_activity: "18 min ago".to_string(),
            last_saved: "Verify sudden spike".to_string(),
            first_seen: "Jun 4, 2026",
            device_uid: "sensor_ec_51bd84".to_string(),
            probe_serial: "ENV-40-EC-K1-4821".to_string(),
            circuit: "Atlas Scientific EZO-EC",
            firmware: "2.14".to_string(),
            knot: "Local Raspberry Pi",
            registered: true,
            calibrated: false,
            calibration_kind: "Two or three point",
            calibration_points: "Dry · 1,413 · 12,880 µS/cm",
            calibration_instruction: "Reconnect the circuit, inspect the probe for deposits, then verify the dry, low, and high calibration points before trusting the sudden spike.",
            last_calibration: "Jun 12, 2026".to_string(),
            next_calibration: "Verification required".to_string(),
        },
    ]
}

#[cfg(not(feature = "demo"))]
fn sensor_from_live(sensor: LiveSensorOverview) -> SensorItem {
    let is_temperature = sensor.kind == "temperature";
    let protocol = if sensor.protocol.eq_ignore_ascii_case("uart") {
        ConnectionProtocol::Uart
    } else {
        ConnectionProtocol::I2c
    };
    let state = if sensor.connected {
        ConnectionState::Connected
    } else {
        ConnectionState::RecentlyDisconnected
    };
    let value = sensor
        .latest_value
        .map(|value| format!("{value:.1}"))
        .unwrap_or_else(|| "—".to_string());
    let device_uid = sensor.device_uid;

    SensorItem {
        key: SensorKey::Live(stable_sensor_key(&device_uid)),
        name: sensor.display_name.unwrap_or_else(|| {
            if is_temperature {
                "Air temperature".to_string()
            } else {
                "Sensor".to_string()
            }
        }),
        role: if is_temperature {
            "Air temperature"
        } else {
            "Environmental sensor"
        },
        metric: if is_temperature {
            "Temperature"
        } else {
            "Raw measurement"
        },
        unit: if is_temperature {
            "Celsius (°C)"
        } else {
            "Raw"
        },
        symbol: if is_temperature { "°C" } else { "—" },
        value,
        state,
        protocol,
        address: sensor.address,
        connection_detail: sensor.connection_detail,
        frequency: measurement_frequency(sensor.measurement_interval_ms),
        last_activity: if sensor.connected {
            "Just now".to_string()
        } else {
            "No recent sample".to_string()
        },
        last_saved: if sensor.latest_value.is_some() {
            "Latest measurement".to_string()
        } else {
            "No measurement".to_string()
        },
        first_seen: "Registered in the local Hub",
        probe_serial: device_uid.clone(),
        device_uid,
        circuit: if is_temperature {
            "Atlas Scientific EZO-RTD"
        } else {
            "Registered sensor"
        },
        firmware: sensor
            .firmware
            .map(|version| format!("{version:.1}"))
            .unwrap_or_else(|| "Unknown".to_string()),
        knot: "Local station",
        registered: true,
        calibrated: false,
        calibration_kind: if is_temperature {
            "Single point"
        } else {
            "Sensor-specific"
        },
        calibration_points: "Not reported by the Hub",
        calibration_instruction:
            "Follow the probe calibration procedure, then confirm the reference measurement.",
        last_calibration: "Not reported".to_string(),
        next_calibration: "Not scheduled".to_string(),
    }
}

#[cfg(not(feature = "demo"))]
fn stable_sensor_key(device_uid: &str) -> u64 {
    device_uid.bytes().fold(0xcbf29ce484222325, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
    })
}

#[cfg(not(feature = "demo"))]
fn measurement_frequency(interval_ms: i32) -> String {
    if interval_ms == 1_000 {
        "1 Hz".to_string()
    } else if interval_ms > 0 && interval_ms % 1_000 == 0 {
        format!("Every {} sec", interval_ms / 1_000)
    } else {
        format!("Every {interval_ms} ms")
    }
}

fn open_sensor(
    sensor: &SensorItem,
    selected_sensor: RwSignal<Option<SensorKey>>,
    draft_name: RwSignal<String>,
    calibration_open: RwSignal<bool>,
    remove_confirmation: RwSignal<bool>,
    action_message: RwSignal<Option<String>>,
) {
    draft_name.set(sensor.name.clone());
    calibration_open.set(false);
    remove_confirmation.set(false);
    action_message.set(None);
    selected_sensor.set(Some(sensor.key));
}

#[component]
pub fn SensorsPage() -> impl IntoView {
    let data_sources = expect_context::<AppDataSources>();
    #[cfg(feature = "demo")]
    let sensors = RwSignal::new(demo_sensors(
        data_sources
            .air_temperature
            .get_untracked()
            .map(|snapshot| snapshot.latest),
        data_sources
            .water_temperature
            .get_untracked()
            .map(|snapshot| snapshot.latest),
        data_sources
            .water_ph
            .get_untracked()
            .map(|snapshot| snapshot.latest),
    ));
    #[cfg(not(feature = "demo"))]
    let sensors = RwSignal::new(Vec::<SensorItem>::new());
    let active_tab = RwSignal::new(SensorTab::Registered);
    let selected_sensor = RwSignal::new(None::<SensorKey>);
    let draft_name = RwSignal::new(String::new());
    let calibration_open = RwSignal::new(false);
    let remove_confirmation = RwSignal::new(false);
    let action_message = RwSignal::new(None::<String>);
    let loading = RwSignal::new(!DEMO_MODE);
    let load_error = RwSignal::new(None::<String>);

    #[cfg(feature = "demo")]
    Effect::watch(
        move || {
            (
                data_sources.air_temperature.get(),
                data_sources.water_temperature.get(),
                data_sources.water_ph.get(),
            )
        },
        move |(air_temperature, water_temperature, water_ph), _, _| {
            sensors.update(|items| {
                for sensor in items {
                    match sensor.key {
                        SensorKey::AirTemperature => {
                            if let Some(snapshot) = air_temperature {
                                sensor.value = format!("{:.1}", snapshot.latest);
                            }
                        }
                        SensorKey::WaterTemperature => {
                            if let Some(snapshot) = water_temperature {
                                sensor.value = format!("{:.1}", snapshot.latest);
                            }
                        }
                        SensorKey::WaterPh => {
                            if let Some(snapshot) = water_ph {
                                sensor.value = format!("{:.2}", snapshot.latest);
                            }
                        }
                        SensorKey::DissolvedOxygen
                        | SensorKey::Conductivity
                        | SensorKey::Live(_) => {}
                    }
                }
            });
        },
        false,
    );

    #[cfg(not(feature = "demo"))]
    Effect::new(move |_| {
        spawn_local(async move {
            match load_live_sensors().await {
                Ok(live_sensors) => {
                    sensors.set(live_sensors.into_iter().map(sensor_from_live).collect());
                    load_error.set(None);
                }
                Err(err) => {
                    load_error.set(Some(format!("Unable to load sensors from the Hub: {err}")));
                }
            }
            loading.set(false);
        });
    });

    let selected = Signal::derive(move || {
        selected_sensor.get().and_then(|key| {
            sensors.with(|items| items.iter().find(|sensor| sensor.key == key).cloned())
        })
    });

    view! {
        <div class="flex h-full min-h-0 flex-col overflow-hidden">
            <header class="flex shrink-0 items-center justify-between gap-4 border-b border-[var(--arksync-panel-border)] bg-[var(--arksync-app-bg)] px-5 py-4 sm:px-8">
                <PageTitle eyebrow="Hardware" title="Sensors" />
            </header>

            <div class="relative min-h-0 flex-1 overflow-hidden">
                <main class="h-full overflow-y-auto px-5 pb-10 pt-7 sm:px-8">
                    <div class="mx-auto max-w-[1480px]">
                        <div class="flex flex-col justify-between gap-5 lg:flex-row lg:items-end">
                            <div>
                                <div class="mb-3 flex items-center gap-2 font-mono text-[10px] uppercase tracking-[0.22em] text-[var(--arksync-action-accent)]">
                                    <span class="h-px w-6 bg-[var(--arksync-action-accent)]"></span>
                                    "Live inventory"
                                </div>
                                <h1 class="text-3xl font-semibold tracking-[-0.035em] text-[var(--arksync-text-strong)] sm:text-4xl">
                                    "Sensors"
                                </h1>
                                <p class="mt-2 max-w-2xl text-sm leading-6 text-[var(--arksync-text-muted)]">
                                    "Inspect live readings, register discovered hardware, and keep calibration metadata close to each probe."
                                </p>
                            </div>

                            <div class="grid grid-cols-3 gap-2 sm:min-w-[430px]">
                                <MetricSummary
                                    label="Registered"
                                    value=Signal::derive(move || sensors.with(|items| items.iter().filter(|sensor| sensor.registered).count()).to_string())
                                />
                                <MetricSummary
                                    label="Online"
                                    value=Signal::derive(move || sensors.with(|items| items.iter().filter(|sensor| sensor.is_connected()).count()).to_string())
                                />
                                <MetricSummary
                                    label="Needs setup"
                                    value=Signal::derive(move || sensors.with(|items| items.iter().filter(|sensor| !sensor.registered).count()).to_string())
                                />
                            </div>
                        </div>

                        <div class="mt-8 border-b border-[var(--arksync-panel-border)]">
                            <div class="flex gap-7" role="tablist" aria-label="Sensor groups">
                                <TabButton
                                    label="Registered"
                                    count=Signal::derive(move || sensors.with(|items| items.iter().filter(|sensor| sensor.registered).count()))
                                    tab=SensorTab::Registered
                                    active_tab=active_tab
                                />
                                <TabButton
                                    label="Detected"
                                    count=Signal::derive(move || sensors.with(|items| items.iter().filter(|sensor| !sensor.registered).count()))
                                    tab=SensorTab::Detected
                                    active_tab=active_tab
                                />
                            </div>
                        </div>

                        {move || loading.get().then(|| view! {
                            <div class="mt-6 rounded-lg border border-[var(--arksync-panel-border)] bg-[var(--arksync-panel-bg)] px-4 py-3 text-xs text-[var(--arksync-text-muted)]">
                                "Loading sensors from the local Hub…"
                            </div>
                        })}
                        {move || load_error.get().map(|message| view! {
                            <div class="mt-6 rounded-lg border border-[var(--arksync-danger)] bg-[var(--arksync-danger-bg)] px-4 py-3 text-xs text-[var(--arksync-text)]">
                                {message}
                            </div>
                        })}

                        <div class="mt-6">
                            {move || {
                                let visible = sensors.get();

                                if active_tab.get() == SensorTab::Registered {
                                    let connected = visible
                                        .iter()
                                        .filter(|sensor| sensor.registered && sensor.is_connected())
                                        .cloned()
                                        .collect::<Vec<_>>();
                                    let disconnected = visible
                                        .iter()
                                        .filter(|sensor| sensor.registered && !sensor.is_connected())
                                        .cloned()
                                        .collect::<Vec<_>>();

                                    view! {
                                        <div class="space-y-7">
                                            <SensorTable
                                                title="Connected sensors"
                                                description="Receiving live readings from the local station."
                                                sensors=connected
                                                muted=false
                                                selected_sensor=selected_sensor
                                                draft_name=draft_name
                                                calibration_open=calibration_open
                                                remove_confirmation=remove_confirmation
                                                action_message=action_message
                                            />
                                            <SensorTable
                                                title="Recently disconnected"
                                                description="Kept for one hour with the last known reading."
                                                sensors=disconnected
                                                muted=true
                                                selected_sensor=selected_sensor
                                                draft_name=draft_name
                                                calibration_open=calibration_open
                                                remove_confirmation=remove_confirmation
                                                action_message=action_message
                                            />
                                        </div>
                                    }.into_any()
                                } else {
                                    let detected = visible
                                        .iter()
                                        .filter(|sensor| !sensor.registered)
                                        .cloned()
                                        .collect::<Vec<_>>();

                                    view! {
                                        <div class="space-y-7">
                                            <SensorTable
                                                title="Ready to configure"
                                                description="Hardware discovered by the station but not registered yet."
                                                sensors=detected
                                                muted=false
                                                selected_sensor=selected_sensor
                                                draft_name=draft_name
                                                calibration_open=calibration_open
                                                remove_confirmation=remove_confirmation
                                                action_message=action_message
                                            />
                                        </div>
                                    }.into_any()
                                }
                            }}
                        </div>
                    </div>
                </main>

                {move || selected.get().map(|sensor| view! {
                    <button
                        type="button"
                        aria-label="Close sensor details"
                        class="absolute inset-0 z-20 cursor-default bg-black/25 backdrop-blur-[1px]"
                        on:click=move |_| selected_sensor.set(None)
                    ></button>
                    <SensorDrawer
                        sensor=sensor
                        sensors=sensors
                        active_tab=active_tab
                        selected_sensor=selected_sensor
                        draft_name=draft_name
                        calibration_open=calibration_open
                        remove_confirmation=remove_confirmation
                        action_message=action_message
                    />
                })}
            </div>
        </div>
    }
}

#[component]
fn MetricSummary(
    #[prop(into)] label: String,
    #[prop(into)] value: Signal<String>,
) -> impl IntoView {
    view! {
        <div class="rounded-lg border border-[var(--arksync-panel-border)] bg-[var(--arksync-panel-bg)] px-4 py-3">
            <div class="text-xl font-semibold tabular-nums text-[var(--arksync-text-strong)]">{value}</div>
            <div class="mt-0.5 font-mono text-[9px] uppercase tracking-[0.16em] text-[var(--arksync-text-muted)]">{label}</div>
        </div>
    }
}

#[component]
fn TabButton(
    #[prop(into)] label: String,
    count: Signal<usize>,
    tab: SensorTab,
    active_tab: RwSignal<SensorTab>,
) -> impl IntoView {
    view! {
        <button
            type="button"
            role="tab"
            aria-selected=move || active_tab.get() == tab
            class=move || format!(
                "relative flex items-center gap-2 pb-3 text-sm font-medium transition-colors {}",
                if active_tab.get() == tab {
                    "text-[var(--arksync-text-strong)]"
                } else {
                    "text-[var(--arksync-text-muted)] hover:text-[var(--arksync-text-strong)]"
                }
            )
            on:click=move |_| active_tab.set(tab)
        >
            <span>{label}</span>
            <span class=move || format!(
                "rounded-full px-2 py-0.5 font-mono text-[9px] {}",
                if active_tab.get() == tab {
                    "bg-[var(--arksync-success-bg)] text-[var(--arksync-success)]"
                } else {
                    "bg-[var(--arksync-panel-bg)] text-[var(--arksync-text-muted)]"
                }
            )>{count}</span>
            <span class=move || format!(
                "absolute inset-x-0 -bottom-px h-0.5 bg-[var(--arksync-action-accent)] transition-opacity {}",
                if active_tab.get() == tab { "opacity-100" } else { "opacity-0" }
            )></span>
        </button>
    }
}

#[component]
fn SensorTable(
    #[prop(into)] title: String,
    #[prop(into)] description: String,
    sensors: Vec<SensorItem>,
    muted: bool,
    selected_sensor: RwSignal<Option<SensorKey>>,
    draft_name: RwSignal<String>,
    calibration_open: RwSignal<bool>,
    remove_confirmation: RwSignal<bool>,
    action_message: RwSignal<Option<String>>,
) -> impl IntoView {
    let sensor_count = sensors.len();
    let is_empty = sensor_count == 0;
    let sensor_label = if sensor_count == 1 {
        "sensor"
    } else {
        "sensors"
    };

    view! {
        <section class=move || if muted { "opacity-70 transition-opacity hover:opacity-100" } else { "" }>
            <div class="mb-3 flex items-end justify-between gap-3">
                <div>
                    <h2 class="text-sm font-semibold text-[var(--arksync-text-strong)]">{title}</h2>
                    <p class="mt-1 text-xs text-[var(--arksync-text-muted)]">{description}</p>
                </div>
                <span class="inline-flex items-center gap-1.5 font-mono text-[9px] uppercase tracking-[0.18em] text-[var(--arksync-text-muted)]">
                    <span class="tabular-nums">{sensor_count}</span>
                    <span>{sensor_label}</span>
                </span>
            </div>

            <div class="overflow-hidden rounded-lg border border-[var(--arksync-panel-border)] bg-[var(--arksync-panel-bg)]">
                {if is_empty {
                    view! {
                        <div class="px-5 py-10 text-center text-sm text-[var(--arksync-text-muted)]">
                            "No sensors in this group."
                        </div>
                    }.into_any()
                } else {
                    view! {
                        <div class="overflow-x-auto">
                            <div class="min-w-[930px]">
                                <div class="grid grid-cols-[minmax(220px,1.45fr)_120px_130px_180px_100px_130px_24px] gap-4 border-b border-[var(--arksync-panel-border)] bg-[var(--arksync-panel-subtle)] px-5 py-3 font-mono text-[9px] uppercase tracking-[0.18em] text-[var(--arksync-text-muted)]">
                                    <span>"Sensor"</span>
                                    <span>"Reading"</span>
                                    <span>"State"</span>
                                    <span>"Connection"</span>
                                    <span>"Frequency"</span>
                                    <span>"Last activity"</span>
                                    <span></span>
                                </div>
                                <div class="divide-y divide-[var(--arksync-panel-border)]">
                                    {sensors.into_iter().map(|sensor| view! {
                                        <SensorRow
                                            sensor=sensor
                                            selected_sensor=selected_sensor
                                            draft_name=draft_name
                                            calibration_open=calibration_open
                                            remove_confirmation=remove_confirmation
                                            action_message=action_message
                                        />
                                    }).collect_view()}
                                </div>
                            </div>
                        </div>
                    }.into_any()
                }}
            </div>
        </section>
    }
}

#[component]
fn SensorRow(
    sensor: SensorItem,
    selected_sensor: RwSignal<Option<SensorKey>>,
    draft_name: RwSignal<String>,
    calibration_open: RwSignal<bool>,
    remove_confirmation: RwSignal<bool>,
    action_message: RwSignal<Option<String>>,
) -> impl IntoView {
    let row_sensor = sensor.clone();
    let selected_key = sensor.key;
    let is_connected = sensor.is_connected();
    let state_label = sensor.state_label();
    let protocol_label = sensor.protocol_label();

    view! {
        <button
            type="button"
            class=move || format!(
                "group grid w-full grid-cols-[minmax(220px,1.45fr)_120px_130px_180px_100px_130px_24px] items-center gap-4 px-5 py-4 text-left transition-colors hover:bg-[var(--arksync-panel-elevated)] focus:outline-none focus-visible:ring-2 focus-visible:ring-inset focus-visible:ring-[var(--arksync-action-accent)] {}",
                if selected_sensor.get() == Some(selected_key) { "bg-[var(--arksync-panel-elevated)]" } else { "" }
            )
            on:click=move |_| open_sensor(
                &row_sensor,
                selected_sensor,
                draft_name,
                calibration_open,
                remove_confirmation,
                action_message,
            )
        >
            <span class="flex min-w-0 items-center gap-3">
                <SensorGlyph symbol=sensor.symbol />
                <span class="min-w-0">
                    <span class="block truncate text-sm font-medium text-[var(--arksync-text-strong)]">{sensor.name.clone()}</span>
                    <span class="mt-1 block truncate font-mono text-[9px] tracking-[0.08em] text-[var(--arksync-text-muted)]">{sensor.device_uid}</span>
                </span>
            </span>
            <span>
                <span class="text-base font-semibold tabular-nums text-[var(--arksync-text-strong)]">{sensor.value}</span>
                <span class="ml-1 text-xs text-[var(--arksync-text-muted)]">{sensor.symbol}</span>
                {(!is_connected).then(|| view! {
                    <span class="mt-0.5 block font-mono text-[8px] uppercase tracking-[0.14em] text-[var(--arksync-text-muted)]">"Last sample"</span>
                })}
            </span>
            <StateBadge connected=is_connected label=state_label />
            <span class="min-w-0">
                <span class="block text-xs font-medium text-[var(--arksync-text)]">{protocol_label}</span>
                <span class="mt-1 block truncate font-mono text-[9px] text-[var(--arksync-text-muted)]">{sensor.address}</span>
            </span>
            <span class="font-mono text-xs tabular-nums text-[var(--arksync-text)]">{sensor.frequency}</span>
            <span>
                <span class="block text-xs text-[var(--arksync-text)]">{sensor.last_activity}</span>
                <span class="mt-1 block font-mono text-[8px] uppercase tracking-[0.12em] text-[var(--arksync-text-muted)]">
                    {if sensor.registered { sensor.last_saved } else { sensor.first_seen.to_string() }}
                </span>
            </span>
            <ChevronRightIcon class="h-4 w-4 text-[var(--arksync-text-muted)] transition-transform group-hover:translate-x-0.5 group-hover:text-[var(--arksync-action-accent)]" />
        </button>
    }
}

#[component]
fn SensorDrawer(
    sensor: SensorItem,
    sensors: RwSignal<Vec<SensorItem>>,
    active_tab: RwSignal<SensorTab>,
    selected_sensor: RwSignal<Option<SensorKey>>,
    draft_name: RwSignal<String>,
    calibration_open: RwSignal<bool>,
    remove_confirmation: RwSignal<bool>,
    action_message: RwSignal<Option<String>>,
) -> impl IntoView {
    let sensor_key = sensor.key;
    let is_connected = sensor.is_connected();
    let was_registered = sensor.registered;
    let save_label = if was_registered {
        "Save changes"
    } else {
        "Register sensor"
    };
    let calibration_label = if sensor.calibrated {
        "Recalibrate"
    } else {
        "Start calibration"
    };
    let sensor_name = sensor.name.clone();
    let calibration_instruction = sensor.calibration_instruction;
    let symbol = sensor.symbol;
    let display_name = sensor.name.clone();
    let registered_label = if sensor.registered {
        "Registered sensor"
    } else {
        "Detected sensor"
    };
    let state_label = sensor.state_label();
    let protocol_label = sensor.protocol_label();
    let device_uid = sensor.device_uid;
    let value = sensor.value;
    let last_activity = sensor.last_activity;
    let role = sensor.role;
    let metric = sensor.metric;
    let unit = sensor.unit;
    let address_label = if sensor.protocol == ConnectionProtocol::Uart {
        "Port"
    } else {
        "I2C address"
    };
    let connection_detail_label = if sensor.protocol == ConnectionProtocol::Uart {
        "Baud rate"
    } else {
        "Bus"
    };
    let address = sensor.address;
    let connection_detail = sensor.connection_detail;
    let knot = sensor.knot;
    let calibrated = sensor.calibrated;
    let calibration_kind = sensor.calibration_kind;
    let calibration_points = sensor.calibration_points;
    let last_calibration = sensor.last_calibration.clone();
    let next_calibration = sensor.next_calibration.clone();
    let circuit = sensor.circuit;
    let firmware = sensor.firmware;
    let probe_serial = sensor.probe_serial;
    let first_seen = sensor.first_seen;

    view! {
        <aside
            aria-label="Sensor details"
            class="absolute inset-y-0 right-0 z-30 flex w-full max-w-[510px] flex-col border-l border-[var(--arksync-panel-border)] bg-[var(--arksync-panel-elevated)] shadow-[-18px_0_45px_var(--arksync-drawer-shadow)]"
        >
            <div class="flex shrink-0 items-start justify-between gap-4 border-b border-[var(--arksync-panel-border)] px-5 py-5 sm:px-6">
                <div class="flex min-w-0 items-center gap-3">
                    <SensorGlyph symbol=symbol />
                    <div class="min-w-0">
                        <div class="mb-1 flex items-center gap-2">
                            <span class="font-mono text-[9px] uppercase tracking-[0.18em] text-[var(--arksync-text-muted)]">
                                {registered_label}
                            </span>
                            <StateBadge connected=is_connected label=state_label />
                        </div>
                        <h2 class="truncate text-lg font-semibold tracking-[-0.02em] text-[var(--arksync-text-strong)]">{display_name}</h2>
                        <p class="mt-1 truncate font-mono text-[9px] tracking-[0.08em] text-[var(--arksync-text-muted)]">{device_uid.clone()}</p>
                    </div>
                </div>
                <button
                    type="button"
                    aria-label="Close drawer"
                    class="inline-flex h-8 w-8 shrink-0 items-center justify-center rounded-md border border-[var(--arksync-panel-border)] text-[var(--arksync-text-muted)] transition-colors hover:bg-[var(--arksync-panel-bg)] hover:text-[var(--arksync-text-strong)]"
                    on:click=move |_| selected_sensor.set(None)
                >
                    <XMarkIcon class="h-4 w-4" />
                </button>
            </div>

            <div class="min-h-0 flex-1 overflow-y-auto px-5 py-6 sm:px-6">
                {move || action_message.get().map(|message| view! {
                    <div class="mb-5 flex items-start gap-3 rounded-lg border border-[var(--arksync-success)] bg-[var(--arksync-success-bg)] px-4 py-3 text-xs leading-5 text-[var(--arksync-text)]">
                        <CheckIcon class="mt-0.5 h-4 w-4 shrink-0 text-[var(--arksync-success)]" />
                        <span>{message}</span>
                    </div>
                })}

                <section class="rounded-xl border border-[var(--arksync-panel-border)] bg-[var(--arksync-panel-bg)] p-5">
                    <div class="flex items-end justify-between gap-3">
                        <div>
                            <div class="font-mono text-[9px] uppercase tracking-[0.18em] text-[var(--arksync-text-muted)]">
                                {if is_connected { "Live reading" } else { "Last known reading" }}
                            </div>
                            <div class="mt-2 flex items-baseline gap-2">
                                <span class="text-4xl font-semibold tabular-nums tracking-[-0.05em] text-[var(--arksync-text-strong)]">{value}</span>
                                <span class="text-sm text-[var(--arksync-text-muted)]">{symbol}</span>
                            </div>
                        </div>
                        <div class="text-right">
                            <div class="flex items-center justify-end gap-1.5 text-xs text-[var(--arksync-success)]">
                                <span class=move || format!("h-1.5 w-1.5 rounded-full {}", if is_connected { "bg-[var(--arksync-success)] animate-pulse" } else { "bg-[var(--arksync-text-muted)]" })></span>
                                {if is_connected { "Streaming" } else { "Offline" }}
                            </div>
                            <div class="mt-1 font-mono text-[9px] text-[var(--arksync-text-muted)]">{last_activity}</div>
                        </div>
                    </div>
                </section>

                <DrawerSection title="Configuration" description="Product-facing settings used across dashboards and automation.">
                    <Field label="Sensor name">
                        <input
                            type="text"
                            prop:value=move || draft_name.get()
                            on:input=move |event| draft_name.set(event_target_value(&event))
                            class="arksync-input"
                        />
                    </Field>
                    <div class="grid gap-4 sm:grid-cols-2">
                        <Field label="Role">
                            <select class="arksync-input">
                                <option selected>{role}</option>
                                <option>"Air temperature"</option>
                                <option>"Water temperature"</option>
                                <option>"Water chemistry"</option>
                                <option>"Dissolved oxygen"</option>
                                <option>"Conductivity"</option>
                            </select>
                        </Field>
                        <Field label="Metric">
                            <select class="arksync-input">
                                <option selected>{metric}</option>
                                <option>"Temperature"</option>
                                <option>"pH"</option>
                                <option>"Dissolved oxygen"</option>
                                <option>"Conductivity"</option>
                            </select>
                        </Field>
                    </div>
                    <div class="grid gap-4 sm:grid-cols-2">
                        <Field label="Unit">
                            <select class="arksync-input">
                                <option selected>{unit}</option>
                                <option>"Celsius (°C)"</option>
                                <option>"pH"</option>
                                <option>"Milligrams per litre (mg/L)"</option>
                                <option>"Microsiemens per centimetre (µS/cm)"</option>
                            </select>
                        </Field>
                        <Field label="Read interval">
                            <select class="arksync-input">
                                <option selected>"Every second"</option>
                                <option>"Every 5 seconds"</option>
                                <option>"Every 30 seconds"</option>
                            </select>
                        </Field>
                    </div>
                </DrawerSection>

                <DrawerSection title="Connection" description="Physical address reported by the local Knot runtime.">
                    <div class="grid grid-cols-2 gap-x-4 gap-y-5 rounded-lg border border-[var(--arksync-panel-border)] bg-[var(--arksync-panel-subtle)] p-4">
                        <DetailItem label="Protocol" value=protocol_label />
                        <DetailItem label=address_label value=address />
                        <DetailItem label=connection_detail_label value=connection_detail />
                        <DetailItem label="Knot" value=knot />
                    </div>
                </DrawerSection>

                <DrawerSection title="Calibration" description="Calibration metadata is stored with the registered sensor.">
                    <div class="rounded-lg border border-[var(--arksync-panel-border)] bg-[var(--arksync-panel-subtle)] p-4">
                        <div class="flex items-start justify-between gap-4">
                            <div class="flex items-center gap-3">
                                <span class="inline-flex h-9 w-9 items-center justify-center rounded-lg bg-[var(--arksync-success-bg)] text-[var(--arksync-success)]">
                                    <BeakerIcon class="h-4 w-4" />
                                </span>
                                <div>
                                    <div class="text-sm font-medium text-[var(--arksync-text-strong)]">
                                        {if calibrated { "Calibration valid" } else { "Calibration required" }}
                                    </div>
                                    <div class="mt-1 text-xs text-[var(--arksync-text-muted)]">{calibration_kind}</div>
                                </div>
                            </div>
                            <span class=if calibrated {
                                "rounded-full bg-[var(--arksync-success-bg)] px-2.5 py-1 font-mono text-[8px] uppercase tracking-[0.14em] text-[var(--arksync-success)]"
                            } else {
                                "rounded-full bg-[var(--arksync-warning-bg)] px-2.5 py-1 font-mono text-[8px] uppercase tracking-[0.14em] text-[var(--arksync-warning)]"
                            }>
                                {if calibrated { "Calibrated" } else { "Not calibrated" }}
                            </span>
                        </div>
                        <div class="mt-4 grid grid-cols-2 gap-4 border-t border-[var(--arksync-panel-border)] pt-4">
                            <DetailItem label="Last calibration" value=last_calibration />
                            <DetailItem label="Next due" value=next_calibration />
                            <div class="col-span-2">
                                <DetailItem label="Calibration points" value=calibration_points />
                            </div>
                        </div>
                        <button
                            type="button"
                            class="mt-4 inline-flex w-full items-center justify-center gap-2 rounded-md border border-[var(--arksync-action-border)] px-3 py-2.5 text-xs font-medium text-[var(--arksync-text-strong)] transition-colors hover:bg-[var(--arksync-panel-elevated)] disabled:cursor-not-allowed disabled:opacity-50"
                            disabled=!is_connected
                            on:click=move |_| {
                                action_message.set(None);
                                calibration_open.set(true);
                            }
                        >
                            <BeakerIcon class="h-4 w-4" />
                            {calibration_label}
                        </button>
                    </div>

                    {move || calibration_open.get().then(|| view! {
                        <div class="mt-3 rounded-lg border border-[var(--arksync-action-accent)] bg-[var(--arksync-success-bg)] p-4">
                            <div class="flex items-center justify-between gap-3">
                                <div class="font-mono text-[9px] uppercase tracking-[0.18em] text-[var(--arksync-success)]">"Calibration mode"</div>
                                <button
                                    type="button"
                                    class="text-xs text-[var(--arksync-text-muted)] hover:text-[var(--arksync-text-strong)]"
                                    on:click=move |_| calibration_open.set(false)
                                >"Cancel"</button>
                            </div>
                            <p class="mt-3 text-xs leading-5 text-[var(--arksync-text)]">{calibration_instruction}</p>
                            <div class="mt-4 h-1.5 overflow-hidden rounded-full bg-[var(--arksync-panel-border)]">
                                <div class="h-full w-1/3 rounded-full bg-[var(--arksync-success)]"></div>
                            </div>
                            <button
                                type="button"
                                class="mt-4 w-full rounded-md bg-[var(--arksync-primary)] px-3 py-2.5 text-xs font-medium text-[var(--arksync-primary-text)] transition-colors hover:bg-[var(--arksync-primary-hover)]"
                                on:click=move |_| {
                                    sensors.update(|items| {
                                        if let Some(sensor) = items.iter_mut().find(|sensor| sensor.key == sensor_key) {
                                            sensor.calibrated = true;
                                            sensor.last_calibration = "Just now".to_string();
                                            sensor.next_calibration = "Sep 1, 2027".to_string();
                                        }
                                    });
                                    calibration_open.set(false);
                                    action_message.set(Some("Calibration completed and metadata updated.".to_string()));
                                }
                            >"Complete calibration"</button>
                        </div>
                    })}
                </DrawerSection>

                <DrawerSection title="Hardware identity" description="Read-only identity received from the Atlas Scientific circuit and probe.">
                    <div class="space-y-4 rounded-lg border border-[var(--arksync-panel-border)] bg-[var(--arksync-panel-subtle)] p-4">
                        <DetailRow label="Circuit" value=circuit />
                        <DetailRow label="Firmware" value=firmware />
                        <DetailRow label="Probe serial" value=probe_serial />
                        <DetailRow label="Device UID" value=device_uid />
                        <DetailRow label="First seen" value=first_seen />
                    </div>
                </DrawerSection>

                {was_registered.then(|| view! {
                    <section class="mt-8 rounded-lg border border-[var(--arksync-danger)] bg-[var(--arksync-danger-bg)] p-4">
                        <h3 class="text-sm font-medium text-[var(--arksync-text-strong)]">"Remove sensor"</h3>
                        <p class="mt-1 text-xs leading-5 text-[var(--arksync-text-muted)]">
                            "The hardware remains discoverable, but its product configuration will be removed."
                        </p>
                        {move || if remove_confirmation.get() {
                            view! {
                                <div class="mt-4 flex gap-2">
                                    <button
                                        type="button"
                                        class="flex-1 rounded-md border border-[var(--arksync-panel-border)] px-3 py-2 text-xs text-[var(--arksync-text)]"
                                        on:click=move |_| remove_confirmation.set(false)
                                    >"Cancel"</button>
                                    <button
                                        type="button"
                                        class="flex-1 rounded-md bg-[var(--arksync-danger)] px-3 py-2 text-xs font-medium text-white"
                                        on:click=move |_| {
                                            sensors.update(|items| {
                                                if let Some(sensor) = items.iter_mut().find(|sensor| sensor.key == sensor_key) {
                                                    sensor.registered = false;
                                                }
                                            });
                                            active_tab.set(SensorTab::Detected);
                                            selected_sensor.set(None);
                                        }
                                    >"Confirm removal"</button>
                                </div>
                            }.into_any()
                        } else {
                            view! {
                                <button
                                    type="button"
                                    class="mt-4 text-xs font-medium text-[var(--arksync-danger)] hover:underline"
                                    on:click=move |_| remove_confirmation.set(true)
                                >"Remove from registry"</button>
                            }.into_any()
                        }}
                    </section>
                })}
            </div>

            <div class="flex shrink-0 items-center justify-between gap-3 border-t border-[var(--arksync-panel-border)] bg-[var(--arksync-panel-bg)] px-5 py-4 sm:px-6">
                <span class="hidden text-xs text-[var(--arksync-text-muted)] sm:inline">
                    {if was_registered { "Unsaved edits stay in this drawer." } else { "Review the settings before registration." }}
                </span>
                <div class="ml-auto flex gap-2">
                    <button
                        type="button"
                        class="rounded-md border border-[var(--arksync-panel-border)] px-4 py-2.5 text-xs font-medium text-[var(--arksync-text)] transition-colors hover:bg-[var(--arksync-panel-elevated)]"
                        on:click=move |_| {
                            draft_name.set(sensor_name.clone());
                            action_message.set(None);
                        }
                    >"Reset"</button>
                    <button
                        type="button"
                        class="rounded-md bg-[var(--arksync-primary)] px-4 py-2.5 text-xs font-medium text-[var(--arksync-primary-text)] transition-colors hover:bg-[var(--arksync-primary-hover)]"
                        on:click=move |_| {
                            let name = draft_name.get_untracked();
                            sensors.update(|items| {
                                if let Some(sensor) = items.iter_mut().find(|sensor| sensor.key == sensor_key) {
                                    sensor.name = name;
                                    sensor.registered = true;
                                }
                            });
                            active_tab.set(SensorTab::Registered);
                            action_message.set(Some(if was_registered {
                                "Sensor name and configuration saved.".to_string()
                            } else {
                                "Sensor registered with its new configuration.".to_string()
                            }));
                        }
                    >{save_label}</button>
                </div>
            </div>
        </aside>
    }
}

#[component]
fn DrawerSection(
    #[prop(into)] title: String,
    #[prop(into)] description: String,
    children: Children,
) -> impl IntoView {
    view! {
        <section class="mt-8">
            <h3 class="text-sm font-semibold text-[var(--arksync-text-strong)]">{title}</h3>
            <p class="mt-1 text-xs leading-5 text-[var(--arksync-text-muted)]">{description}</p>
            <div class="mt-4 space-y-4">{children()}</div>
        </section>
    }
}

#[component]
fn Field(#[prop(into)] label: String, children: Children) -> impl IntoView {
    view! {
        <label class="block">
            <span class="mb-1.5 block font-mono text-[9px] uppercase tracking-[0.16em] text-[var(--arksync-text-muted)]">{label}</span>
            {children()}
        </label>
    }
}

#[component]
fn DetailItem(#[prop(into)] label: String, #[prop(into)] value: String) -> impl IntoView {
    view! {
        <div class="min-w-0">
            <div class="font-mono text-[8px] uppercase tracking-[0.15em] text-[var(--arksync-text-muted)]">{label}</div>
            <div class="mt-1 truncate text-xs font-medium text-[var(--arksync-text-strong)]">{value}</div>
        </div>
    }
}

#[component]
fn DetailRow(#[prop(into)] label: String, #[prop(into)] value: String) -> impl IntoView {
    view! {
        <div class="flex items-start justify-between gap-5">
            <span class="font-mono text-[8px] uppercase tracking-[0.15em] text-[var(--arksync-text-muted)]">{label}</span>
            <span class="max-w-[65%] break-all text-right text-xs font-medium text-[var(--arksync-text-strong)]">{value}</span>
        </div>
    }
}

#[component]
fn StateBadge(connected: bool, #[prop(into)] label: String) -> impl IntoView {
    view! {
        <span class=if connected {
            "inline-flex w-fit items-center gap-1.5 rounded-full bg-[var(--arksync-success-bg)] px-2.5 py-1 font-mono text-[8px] uppercase tracking-[0.12em] text-[var(--arksync-success)]"
        } else {
            "inline-flex w-fit items-center gap-1.5 rounded-full bg-[var(--arksync-panel-subtle)] px-2.5 py-1 font-mono text-[8px] uppercase tracking-[0.12em] text-[var(--arksync-text-muted)]"
        }>
            <span class=if connected { "h-1.5 w-1.5 rounded-full bg-[var(--arksync-success)]" } else { "h-1.5 w-1.5 rounded-full bg-[var(--arksync-text-muted)]" }></span>
            {label}
        </span>
    }
}

#[component]
fn SensorGlyph(#[prop(into)] symbol: String) -> impl IntoView {
    view! {
        <span class="inline-flex h-10 w-10 shrink-0 items-center justify-center rounded-lg border border-[var(--arksync-panel-border)] bg-[var(--arksync-panel-subtle)] font-mono text-[11px] font-semibold text-[var(--arksync-action-accent)]">
            {symbol}
        </span>
    }
}

#[component]
fn ChevronRightIcon(#[prop(into)] class: String) -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class=class>
            <path stroke-linecap="round" stroke-linejoin="round" d="m9 18 6-6-6-6" />
        </svg>
    }
}

#[component]
fn XMarkIcon(#[prop(into)] class: String) -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class=class>
            <path stroke-linecap="round" stroke-linejoin="round" d="M6 18 18 6M6 6l12 12" />
        </svg>
    }
}

#[component]
fn BeakerIcon(#[prop(into)] class: String) -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class=class>
            <path stroke-linecap="round" stroke-linejoin="round" d="M14.25 6.087c0-.355.186-.676.401-.959.323-.426.349-1.05.046-1.492-.303-.441-.92-.62-1.422-.436l-2.55.936a1.875 1.875 0 0 1-1.293 0l-2.55-.936c-.503-.184-1.12-.005-1.423.436-.303.442-.277 1.066.046 1.492.215.283.401.604.401.959v.839c0 .67-.27 1.312-.75 1.78l-3.8 3.704a4.5 4.5 0 0 0-1.356 3.22v.12A4.25 4.25 0 0 0 4.25 20h11.5A4.25 4.25 0 0 0 20 15.75v-.12a4.5 4.5 0 0 0-1.356-3.22l-3.644-3.55a2.5 2.5 0 0 1-.75-1.79v-.983Z" />
            <path stroke-linecap="round" stroke-linejoin="round" d="M6.75 13.5h10.5" />
        </svg>
    }
}

#[component]
fn CheckIcon(#[prop(into)] class: String) -> impl IntoView {
    view! {
        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class=class>
            <path stroke-linecap="round" stroke-linejoin="round" d="m4.5 12.75 6 6 9-13.5" />
        </svg>
    }
}
