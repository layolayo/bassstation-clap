use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use nih_plug::prelude::*;
use nih_plug_egui::egui::{self, Color32, RichText};
use nih_plug_egui::{create_egui_editor, EguiState};

use crate::midi_worker::{list_midi_ports, split_syx_messages, SharedMidiState, WorkerCmd};
use crate::params::{
    BassStationParams, CC_ALL_NOTES_OFF, CC_ENV1_ATTACK, CC_ENV1_DECAY, CC_ENV2_ATTACK,
    CC_ENV2_DECAY, CC_FILTER_FREQ, CC_FILTER_MOD_DEPTH, CC_FILTER_RES, CC_MOD_WHEEL, CC_VOLUME,
};
use crate::ui_widgets::{draw_ad_env_graph, draw_filter_graph};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Controls,
    SysexBackup,
    MidiLog,
}

struct EditorUiState {
    tab: Tab,
    out_ports: Vec<String>,
    in_ports: Vec<String>,
    active_patch: u8,
    syx_filename: String,
    syx_files: Vec<String>,
    syx_status: String,
}

impl Default for EditorUiState {
    fn default() -> Self {
        let (out_ports, in_ports) = list_midi_ports();
        let mut st = Self {
            tab: Tab::Controls,
            out_ports,
            in_ports,
            active_patch: 0,
            syx_filename: "bassstation_patch.syx".to_string(),
            syx_files: Vec::new(),
            syx_status: String::new(),
        };
        st.refresh_syx_list();
        st
    }
}

impl EditorUiState {
    fn patch_dir() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(home).join("BassStation_Patches")
    }

    fn refresh_syx_list(&mut self) {
        let dir = Self::patch_dir();
        let _ = fs::create_dir_all(&dir);
        let mut files = Vec::new();
        if let Ok(entries) = fs::read_dir(&dir) {
            for entry in entries.flatten() {
                if let Some(name) = entry.file_name().to_str() {
                    if name.to_lowercase().ends_with(".syx") {
                        files.push(name.to_string());
                    }
                }
            }
        }
        files.sort();
        self.syx_files = files;
    }
}

pub fn create(
    params: Arc<BassStationParams>,
    editor_state: Arc<EguiState>,
    midi_state: SharedMidiState,
    cmd_tx: crossbeam_channel::Sender<WorkerCmd>,
) -> Option<Box<dyn Editor>> {
    let ui_state = Arc::new(Mutex::new(EditorUiState::default()));

    create_egui_editor(
        editor_state,
        (),
        |_, _| {},
        move |egui_ctx, setter, _state| {
            egui_ctx.request_repaint_after(std::time::Duration::from_millis(120));
            apply_incoming_ccs(&params, &midi_state, setter);

            let mut ui_st = ui_state.lock().unwrap();
            let ch = (params.midi_channel.value() - 1).clamp(0, 15) as u8;

            let status = midi_state.status.lock().unwrap().clone();
            let status_color = if status.contains("Err") || status == "Disconnected" {
                Color32::from_rgb(245, 130, 110)
            } else {
                Color32::from_rgb(130, 225, 155)
            };

            egui::TopBottomPanel::bottom("bs_footer").show(egui_ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(format!("MIDI: {}", status))
                            .small()
                            .color(status_color),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format!(
                                "v{} ({})",
                                env!("CARGO_PKG_VERSION"),
                                env!("BS_BUILD_TIME")
                            ))
                            .small()
                            .color(Color32::GRAY),
                        );
                    });
                });
            });

            egui::CentralPanel::default().show(egui_ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new("NOVATION BASS STATION (1994)")
                            .strong()
                            .color(Color32::from_rgb(110, 200, 255)),
                    );

                    let sel_out = midi_state.selected_out.lock().unwrap().clone();
                    egui::ComboBox::from_id_salt("bs_midi_out")
                        .selected_text(if sel_out.is_empty() {
                            "MIDI Out..."
                        } else {
                            &sel_out
                        })
                        .width(150.0)
                        .show_ui(ui, |ui| {
                            for port in &ui_st.out_ports {
                                if ui.selectable_label(sel_out == *port, port).clicked() {
                                    if let Ok(mut s) = params.saved_midi_out.lock() {
                                        *s = port.clone();
                                    }
                                    let _ = cmd_tx.send(WorkerCmd::ConnectOut(port.clone()));
                                }
                            }
                        });

                    let sel_in = midi_state.selected_in.lock().unwrap().clone();
                    egui::ComboBox::from_id_salt("bs_midi_in")
                        .selected_text(if sel_in.is_empty() {
                            "MIDI In..."
                        } else {
                            &sel_in
                        })
                        .width(150.0)
                        .show_ui(ui, |ui| {
                            for port in &ui_st.in_ports {
                                if ui.selectable_label(sel_in == *port, port).clicked() {
                                    if let Ok(mut s) = params.saved_midi_in.lock() {
                                        *s = port.clone();
                                    }
                                    let _ = cmd_tx.send(WorkerCmd::ConnectIn(port.clone()));
                                }
                            }
                        });

                    if ui.button("Refresh").clicked() {
                        let (outs, ins) = list_midi_ports();
                        ui_st.out_ports = outs;
                        ui_st.in_ports = ins;
                    }

                    ui.label("Ch:");
                    let mut ch_val = params.midi_channel.value();
                    if ui
                        .add(egui::DragValue::new(&mut ch_val).range(1..=16))
                        .changed()
                    {
                        setter.begin_set_parameter(&params.midi_channel);
                        setter.set_parameter(&params.midi_channel, ch_val);
                        setter.end_set_parameter(&params.midi_channel);
                    }

                    if ui.button("Panic").clicked() {
                        let _ = cmd_tx
                            .send(WorkerCmd::SendMidi3([0xB0 | ch, CC_ALL_NOTES_OFF, 0], 3));
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Patch (PC 0-6):");
                    for p in 0..7u8 {
                        if ui
                            .selectable_label(ui_st.active_patch == p, format!("P{}", p + 1))
                            .clicked()
                        {
                            ui_st.active_patch = p;
                            let _ = cmd_tx.send(WorkerCmd::SendMidi3([0xC0 | ch, p, 0], 2));
                        }
                    }
                });
                ui.separator();

                ui.horizontal(|ui| {
                    ui.selectable_value(&mut ui_st.tab, Tab::Controls, "Controls & Envelopes");
                    ui.selectable_value(&mut ui_st.tab, Tab::SysexBackup, "SysEx & Backup (.syx)");
                    ui.selectable_value(&mut ui_st.tab, Tab::MidiLog, "MIDI Log");
                });
                ui.separator();

                match ui_st.tab {
                    Tab::Controls => draw_controls_tab(ui, &params, setter),
                    Tab::SysexBackup => draw_sysex_tab(ui, &midi_state, &cmd_tx, &mut ui_st),
                    Tab::MidiLog => draw_log_tab(ui, &midi_state),
                }
            });
        },
    )
}

fn draw_controls_tab(ui: &mut egui::Ui, params: &BassStationParams, setter: &ParamSetter) {
    ui.columns(3, |cols| {
        cols[0].group(|ui| {
            ui.set_min_height(250.0);
            ui.heading("Filter & Output");
            draw_filter_graph(
                ui,
                params.filter_freq.value(),
                params.filter_res.value(),
                params.filter_mod_depth.value(),
            );
            int_row(ui, "Freq (CC 105)", &params.filter_freq, 0..=127, setter);
            int_row(ui, "Res (CC 106)", &params.filter_res, 0..=127, setter);
            int_row(ui, "Mod (CC 107)", &params.filter_mod_depth, 0..=127, setter);
            ui.separator();
            int_row(ui, "MW (CC 01)", &params.mod_wheel, 0..=127, setter);
            ui.horizontal(|ui| {
                int_row(ui, "Pitch Bend", &params.pitch_bend, -64..=63, setter);
                if ui.button("0").on_hover_text("Reset Pitch Bend").clicked() {
                    setter.begin_set_parameter(&params.pitch_bend);
                    setter.set_parameter(&params.pitch_bend, 0);
                    setter.end_set_parameter(&params.pitch_bend);
                }
            });
            int_row(ui, "Vol (CC 07)", &params.volume, 0..=127, setter);
        });

        cols[1].group(|ui| {
            ui.set_min_height(250.0);
            ui.heading("Envelope 1");
            draw_ad_env_graph(
                ui,
                params.env1_attack.value(),
                params.env1_decay.value(),
                Color32::from_rgb(130, 230, 180),
            );
            int_row(ui, "Attack (CC 108)", &params.env1_attack, 0..=127, setter);
            int_row(ui, "Decay (CC 109)", &params.env1_decay, 0..=127, setter);
            ui.add_space(8.0);
            ui.label(
                RichText::new("Sustain / Release / Autoglide are panel-only on 1994 v1.4")
                    .small()
                    .color(Color32::GRAY),
            );
        });

        cols[2].group(|ui| {
            ui.set_min_height(250.0);
            ui.heading("Envelope 2 (Filter)");
            draw_ad_env_graph(
                ui,
                params.env2_attack.value(),
                params.env2_decay.value(),
                Color32::from_rgb(245, 175, 85),
            );
            int_row(ui, "Attack (CC 114)", &params.env2_attack, 0..=127, setter);
            int_row(ui, "Decay (CC 115)", &params.env2_decay, 0..=127, setter);
        });
    });
}

fn draw_sysex_tab(
    ui: &mut egui::Ui,
    midi_state: &SharedMidiState,
    cmd_tx: &crossbeam_channel::Sender<WorkerCmd>,
    ui_st: &mut EditorUiState,
) {
    let dir = EditorUiState::patch_dir();
    ui.label(format!("SysEx Folder: {}", dir.display()));

    let (cap_count, cap_bytes) = {
        let cap = midi_state.captured_sysex.lock().unwrap();
        let bytes: usize = cap.iter().map(|d| d.len()).sum();
        (cap.len(), bytes)
    };

    ui.horizontal(|ui| {
        ui.label("Filename:");
        let resp =
            ui.add(egui::TextEdit::singleline(&mut ui_st.syx_filename).desired_width(180.0));
        if resp.has_focus()
            && ui.input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Escape))
        {
            resp.surrender_focus();
        }
        if ui.button("Clear Capture").clicked() {
            midi_state.captured_sysex.lock().unwrap().clear();
            ui_st.syx_status = "Capture cleared. Trigger SysEx dump on Bass Station.".to_string();
        }
        if ui.button("Save Capture .syx").clicked() {
            let cap = midi_state.captured_sysex.lock().unwrap();
            if cap.is_empty() {
                ui_st.syx_status = "Nothing captured yet!".to_string();
            } else {
                let mut combined = Vec::new();
                for d in cap.iter() {
                    combined.extend_from_slice(d);
                }
                let path = dir.join(&ui_st.syx_filename);
                ui_st.syx_status = match fs::write(&path, &combined) {
                    Ok(_) => {
                        ui_st.refresh_syx_list();
                        format!("Saved {} msg(s) ({} bytes) to {}", cap.len(), combined.len(), path.display())
                    }
                    Err(e) => format!("Save error: {}", e),
                };
            }
        }
        if ui.button("Refresh List").clicked() {
            ui_st.refresh_syx_list();
        }
    });

    ui.label(format!("Captured: {} msg(s), {} bytes", cap_count, cap_bytes));
    if !ui_st.syx_status.is_empty() {
        ui.label(RichText::new(&ui_st.syx_status).color(Color32::LIGHT_GREEN));
    }

    ui.separator();
    ui.heading("Saved .syx Files (Click to Send to Bass Station)");
    egui::ScrollArea::vertical().show(ui, |ui| {
        for fname in ui_st.syx_files.clone() {
            if ui.button(&fname).clicked() {
                let path = dir.join(&fname);
                match fs::read(&path) {
                    Ok(bytes) => {
                        let msgs = split_syx_messages(&bytes);
                        let count = msgs.len();
                        let _ = cmd_tx.send(WorkerCmd::SendRawSysex(msgs));
                        ui_st.syx_status = format!("Sent {} SysEx msg(s) from {}", count, fname);
                    }
                    Err(e) => ui_st.syx_status = format!("Read error: {}", e),
                }
            }
        }
    });
}

fn draw_log_tab(ui: &mut egui::Ui, midi_state: &SharedMidiState) {
    let all_text = {
        let logs = midi_state.logs.lock().unwrap();
        logs.iter().cloned().collect::<Vec<_>>().join("\n")
    };
    ui.horizontal(|ui| {
        if ui.button("Copy All").clicked() {
            ui.ctx().copy_text(all_text.clone());
        }
        if ui.button("Clear").clicked() {
            if let Ok(mut logs) = midi_state.logs.lock() {
                logs.clear();
            }
        }
    });
    ui.separator();
    egui::ScrollArea::vertical().show(ui, |ui| {
        let logs = midi_state.logs.lock().unwrap();
        for line in logs.iter().rev() {
            ui.monospace(line);
        }
    });
}

fn apply_incoming_ccs(
    params: &BassStationParams,
    midi_state: &SharedMidiState,
    setter: &ParamSetter,
) {
    let Ok(mut q) = midi_state.incoming_ccs.lock() else {
        return;
    };
    while let Some((cc, val)) = q.pop_front() {
        let target = match cc {
            CC_MOD_WHEEL => Some(&params.mod_wheel),
            CC_VOLUME => Some(&params.volume),
            CC_FILTER_FREQ => Some(&params.filter_freq),
            CC_FILTER_RES => Some(&params.filter_res),
            CC_FILTER_MOD_DEPTH => Some(&params.filter_mod_depth),
            CC_ENV1_ATTACK => Some(&params.env1_attack),
            CC_ENV1_DECAY => Some(&params.env1_decay),
            CC_ENV2_ATTACK => Some(&params.env2_attack),
            CC_ENV2_DECAY => Some(&params.env2_decay),
            _ => None,
        };
        if let Some(p) = target {
            midi_state.suppress_tx_for(200);
            setter.begin_set_parameter(p);
            setter.set_parameter(p, val as i32);
            setter.end_set_parameter(p);
        }
    }
}

fn int_row(
    ui: &mut egui::Ui,
    label: &str,
    param: &IntParam,
    range: std::ops::RangeInclusive<i32>,
    setter: &ParamSetter,
) {
    let mut val = param.value();
    ui.horizontal(|ui| {
        ui.add_sized([105.0, 18.0], egui::Label::new(label));
        if ui.add(egui::Slider::new(&mut val, range)).changed() {
            setter.begin_set_parameter(param);
            setter.set_parameter(param, val);
            setter.end_set_parameter(param);
        }
    });
}
