use std::panic;

use sycamore::prelude::*;

use sycamore::futures::{spawn_local, spawn_local_scoped};

use gloo_timers::future::TimeoutFuture;

pub mod js_glue;

pub mod api;
use api::*;

pub mod trace;
use trace::*;

pub mod graph;
use graph::*;

pub mod side_bar;
use side_bar::*;

fn main() {
    panic::set_hook(Box::new(|reason| {
        let reason = format!("PANIC!!!! -> {}", reason);
        spawn_local(async move {
            print_backend(&reason).await;
        });
    }));

    sycamore::render(|cx| {
        view! { cx,
            Main {}
        }
    })
}

async fn get_trace_info() -> DrawInfo {
    let svg_size = get_svg_size().await;
    let wavelength_limits = get_wavelength_limits().await;
    let power_limits = get_power_limits().await;
    let valley_detection = get_valley_detection().await;
    let peak_detection = get_peak_detection().await;
    let time_series_config = get_time_series_config().await;

    DrawInfo {
        svg_size,
        wavelength_limits,
        power_limits,
        valley_detection,
        peak_detection,
        time_series_config,
    }
}

pub enum ActiveSide {
    Traces,
    Config,
}

// COMPONENTS ----------------------------

#[component]
fn Main<G: Html>(cx: Scope) -> View<G> {
    let traces = create_signal(cx, vec![first_trace()]);

    let shadow_paths_info = create_signal(cx, empty_draw_info());
    let shadow_paths = create_signal(cx, vec!["".to_string()]);
    let draw_shadow = create_signal(cx, false);

    let time_series_paths_info = create_signal(cx, empty_draw_info());
    let time_series_paths = create_signal(cx, TimeSeriesGroupPaths::empty());
    let draw_time_series = create_signal(cx, false);

    let svg_size = create_signal(cx, (0i32, 0i32));

    // Get new spectra
    spawn_local_scoped(cx, async move {
        loop {
            TimeoutFuture::new(200).await; // 5 fps, #TODO send to config
            let current_info = get_trace_info().await;

            if unread_spectrum().await {
                // Get the latest spectrum if it is available
                let new_path = get_last_spectrum_path().await;
                let new_valleys = get_last_spectrum_valleys_points().await;
                let new_peaks = get_last_spectrum_peaks_points().await;

                if let Some(trace) = traces.modify().last_mut() {
                    trace.drawn_info = current_info.clone();
                    trace.svg_path = new_path;
                    trace.valleys = new_valleys;
                    trace.peaks = new_peaks;
                };

                // Also update the shadow
                shadow_paths_info.set(current_info.clone());
                shadow_paths.set(get_shadow_paths().await);

                // And the time series
                if *draw_time_series.get() {
                    time_series_paths_info.set(current_info.clone());
                    time_series_paths.set(get_time_series_paths().await);
                }
            }
        }
    });

    // Update on window / config / info update
    spawn_local_scoped(cx, async move {
        loop {
            TimeoutFuture::new(100).await; // 10 fps, TODO send to config / use as event
            let current_info = get_trace_info().await;

            let new_svg_size = get_svg_size().await;
            'traces: for (id, trace) in traces.modify().iter_mut().enumerate() {
                // Update when the window changes
                if trace.svg_path.is_empty() {
                    // No old spectrum, no update
                    continue 'traces;
                }
                if trace.drawn_info != current_info {
                    trace.drawn_info = current_info.clone();
                    if trace.active {
                        trace.svg_path = get_last_spectrum_path().await;
                        trace.valleys = get_last_spectrum_valleys_points().await;
                        trace.peaks = get_last_spectrum_peaks_points().await;
                    } else {
                        trace.svg_path = get_frozen_spectrum_path(id).await;
                        trace.valleys = get_frozen_spectrum_valleys_points(id).await;
                        trace.peaks = get_frozen_spectrum_peaks_points(id).await;
                    }
                }
            }

            // Also update the shadows
            if *shadow_paths_info.get() != current_info {
                shadow_paths_info.set(current_info.clone());
                shadow_paths.set(get_shadow_paths().await);
            };

            svg_size.set(new_svg_size);

            // And the time series
            if (*draw_time_series.get()) && (*time_series_paths_info.get() != current_info) {
                time_series_paths_info.set(current_info.clone());
                time_series_paths.set(get_time_series_paths().await);
            };
        }
    });

    // Update time series when activated
    create_effect(cx, move || {
        if *draw_time_series.get() {
            spawn_local_scoped(cx, async move {
                time_series_paths_info.set(get_trace_info().await);
                time_series_paths.set(get_time_series_paths().await);
            });
        }
    });

    let saving = create_signal(cx, false);
    let connection_state = create_signal(cx, ConnectionState::Disconnected);

    spawn_local_scoped(cx, async move {
        loop {
            TimeoutFuture::new(200).await; // 5 fps, #TODO send to config / use as event
            update_state(connection_state).await;
        }
    });

    // Some interconnecting signals (a bit spaghetti, maybe TODO fix this?)
    let active_side = create_signal(cx, ActiveSide::Traces);
    let limits_change_flag = create_signal(cx, false);
    let series_total_time = create_signal(cx, 0);
    spawn_local_scoped(cx, async move {
        let config = get_handler_config().await;
        series_total_time.set(config.time_series_config.total_time as i32);
    });

    view! { cx,
        div(class="horizontal-container") {

            SideBar(
                traces=traces,
                saving=saving,
                active_side=active_side,
                limits_change_flag=limits_change_flag,
                draw_shadow=draw_shadow,
                draw_time_series=draw_time_series,
                series_total_time=series_total_time
            )

            div(class="vertical-container") {
                Graph(
                    traces=traces,
                    svg_size=svg_size,
                    shadow_paths=shadow_paths,
                    time_series_paths=time_series_paths,
                    limits_change_flag=limits_change_flag,
                    draw_shadow=draw_shadow,
                    draw_time_series=draw_time_series,
                    series_total_time=series_total_time
                )

                LowerBar(
                    saving=saving,
                    connection_state=connection_state,
                    active_side=active_side
                )
            }
        }
    }
}

async fn update_state(connection_state: &Signal<ConnectionState>) {
    if let Some(new_state) = get_connection_state().await {
        if new_state != *connection_state.get() {
            connection_state.set(new_state);
        }
    }
}

#[derive(Prop)]
struct LowerBarProps<'a> {
    saving: &'a ReadSignal<bool>,
    connection_state: &'a Signal<ConnectionState>,
    active_side: &'a Signal<ActiveSide>,
}

#[component]
fn LowerBar<'a, G: Html>(cx: Scope<'a>, props: LowerBarProps<'a>) -> View<G> {
    let connect = move |_| {
        spawn_local_scoped(cx, async move {
            connect_acquisitor().await;
            update_state(props.connection_state).await;
        })
    };
    let disconnect = move |_| {
        spawn_local_scoped(cx, async move {
            disconnect_acquisitor().await;
            update_state(props.connection_state).await;
        })
    };
    let start_reading = move |_| {
        spawn_local_scoped(cx, async move {
            acquisitor_start_reading().await;
            update_state(props.connection_state).await;
        })
    };
    let read_single = move |_| {
        spawn_local_scoped(cx, async move {
            acquisitor_read_single().await;
            update_state(props.connection_state).await;
        })
    };
    let stop_reading = move |_| {
        spawn_local_scoped(cx, async move {
            acquisitor_stop_reading().await;
            update_state(props.connection_state).await;
        })
    };

    let enter_config = move |_| match *props.active_side.get() {
        ActiveSide::Config => (),
        _ => {
            props.active_side.set(ActiveSide::Config);
        }
    };

    let enter_traces = move |_| match *props.active_side.get() {
        ActiveSide::Traces => (),
        _ => {
            props.active_side.set(ActiveSide::Traces);
        }
    };

    view! { cx,
        div(class="lower-bar back") {
            div() {
                button(on:click=enter_traces, title="Traços") { "󰽉 "}
                button(on:click=enter_config, title="Configurações") { "󰢻 "}
            }
            div(class="csv-front") {
                h2 { "CSV" }
            }
            div() {
                (match *props.connection_state.get() {
                    ConnectionState::Connected =>
                        view! { cx,
                            button(on:click=start_reading, class="no-offset", title="Ler continuamente") { " " }
                            button(on:click=read_single, class="no-offset", title="Ler 1 espectro") { " " }
                            button(on:click=disconnect, title="Desconectar aquisitor") { "󱐤 " }
                        },
                    ConnectionState::Reading =>
                        view! { cx,
                            button(on:click=stop_reading, class="no-offset", title="Interromper leitura") { " " }
                            button(on:click=disconnect, title="Desconectar aquisitor") { "󱐤 " }
                        },
                    ConnectionState::Disconnected =>
                        view! { cx,
                            button(on:click=connect, class="no-offset", title="Conectar acquisitor") { "󱐥 " }
                        }
                })
            }
            Status(saving=props.saving, connection_state=props.connection_state)
        }
    }
}

#[derive(Prop)]
struct StatusProps<'a> {
    saving: &'a ReadSignal<bool>,
    connection_state: &'a ReadSignal<ConnectionState>,
}

#[component]
fn Status<'a, G: Html>(cx: Scope<'a>, props: StatusProps<'a>) -> View<G> {
    view! { cx,
        div(class="status") {
            (match *props.connection_state.get() {
                ConnectionState::Connected =>
                    view! { cx, p() { "Conectado" } },
                ConnectionState::Disconnected =>
                    view! { cx, p() { "Desconectado" } },
                ConnectionState::Reading =>
                    view! { cx, p() { "Lendo" } }
            })

            (if *props.saving.get() {
                view! { cx,
                    p() { "Salvando" }
                }
            } else {
                view! { cx,
                    p() { "Não Salvando" }
                }
            })
        }
    }
}
