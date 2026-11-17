use std::num::NonZeroU32;
use std::sync::Arc;

use nih_plug::prelude::*;
use nih_plug_egui::EguiState;

mod editor;
mod midi_worker;
mod params;
mod ui_widgets;

use midi_worker::{MidiWorker, WorkerCmd};
use params::{
    BassStationParams, CC_ENV1_ATTACK, CC_ENV1_DECAY, CC_ENV2_ATTACK, CC_ENV2_DECAY,
    CC_FILTER_FREQ, CC_FILTER_MOD_DEPTH, CC_FILTER_RES, CC_MOD_WHEEL, CC_VOLUME,
};

pub struct BassStationPlugin {
    params: Arc<BassStationParams>,
    editor_state: Arc<EguiState>,
    midi_worker: MidiWorker,
    last_cc: [i32; 9],
    last_pb: i32,
    shadows_initialized: bool,
}

impl Default for BassStationPlugin {
    fn default() -> Self {
        Self {
            params: Arc::new(BassStationParams::default()),
            editor_state: EguiState::from_size(920, 430),
            midi_worker: MidiWorker::spawn(),
            last_cc: [-1; 9],
            last_pb: i32::MIN,
            shadows_initialized: false,
        }
    }
}

impl BassStationPlugin {
    fn sync_params(&mut self) {
        let ch = (self.params.midi_channel.value() - 1).clamp(0, 15) as u8;
        let cc_vals: [(u8, i32); 9] = [
            (CC_MOD_WHEEL, self.params.mod_wheel.value()),
            (CC_VOLUME, self.params.volume.value()),
            (CC_FILTER_FREQ, self.params.filter_freq.value()),
            (CC_FILTER_RES, self.params.filter_res.value()),
            (CC_FILTER_MOD_DEPTH, self.params.filter_mod_depth.value()),
            (CC_ENV1_ATTACK, self.params.env1_attack.value()),
            (CC_ENV1_DECAY, self.params.env1_decay.value()),
            (CC_ENV2_ATTACK, self.params.env2_attack.value()),
            (CC_ENV2_DECAY, self.params.env2_decay.value()),
        ];

        for (i, &(cc, val)) in cc_vals.iter().enumerate() {
            if self.last_cc[i] != val {
                self.last_cc[i] = val;
                if self.shadows_initialized && !self.midi_worker.state.is_tx_suppressed() {
                    self.midi_worker
                        .try_send(WorkerCmd::SendMidi3([0xB0 | ch, cc, val as u8], 3));
                }
            }
        }

        let pb = self.params.pitch_bend.value().clamp(-64, 63);
        if self.last_pb != pb {
            self.last_pb = pb;
            if self.shadows_initialized && !self.midi_worker.state.is_tx_suppressed() {
                let msb = (pb + 64).clamp(0, 127) as u8;
                let lsb = if pb == 63 { 127 } else { 0 };
                self.midi_worker
                    .try_send(WorkerCmd::SendMidi3([0xE0 | ch, lsb, msb], 3));
            }
        }
        self.shadows_initialized = true;
    }
}

impl Plugin for BassStationPlugin {
    const NAME: &'static str = "Novation BassStation (1994)";
    const VENDOR: &'static str = "BassStation-CLAP";
    const URL: &'static str = "";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::Basic;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        editor::create(
            self.params.clone(),
            self.editor_state.clone(),
            self.midi_worker.state.clone(),
            self.midi_worker.tx.clone(),
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        if let Ok(out) = self.params.saved_midi_out.lock() {
            if !out.is_empty() {
                self.midi_worker.try_send(WorkerCmd::ConnectOut(out.clone()));
            }
        }
        if let Ok(inp) = self.params.saved_midi_in.lock() {
            if !inp.is_empty() {
                self.midi_worker.try_send(WorkerCmd::ConnectIn(inp.clone()));
            }
        }
        self.sync_params();
        true
    }

    fn process(
        &mut self,
        _buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let ch = (self.params.midi_channel.value() - 1).clamp(0, 15) as u8;

        while let Some(event) = context.next_event() {
            match event {
                NoteEvent::NoteOn { note, velocity, .. } => {
                    let vel = (velocity * 127.0).round().clamp(1.0, 127.0) as u8;
                    self.midi_worker
                        .try_send(WorkerCmd::SendMidi3([0x90 | ch, note & 0x7F, vel], 3));
                }
                NoteEvent::NoteOff { note, velocity, .. } => {
                    let vel = (velocity * 127.0).round().clamp(0.0, 127.0) as u8;
                    self.midi_worker
                        .try_send(WorkerCmd::SendMidi3([0x80 | ch, note & 0x7F, vel], 3));
                }
                NoteEvent::MidiCC { cc, value, .. } => {
                    let val = (value * 127.0).round().clamp(0.0, 127.0) as u8;
                    self.midi_worker
                        .try_send(WorkerCmd::SendMidi3([0xB0 | ch, cc & 0x7F, val], 3));
                }
                NoteEvent::MidiPitchBend { value, .. } => {
                    let bend = (value * 16383.0).round().clamp(0.0, 16383.0) as u16;
                    self.midi_worker.try_send(WorkerCmd::SendMidi3(
                        [0xE0 | ch, (bend & 0x7F) as u8, ((bend >> 7) & 0x7F) as u8],
                        3,
                    ));
                }
                NoteEvent::MidiProgramChange { program, .. } => {
                    self.midi_worker
                        .try_send(WorkerCmd::SendMidi3([0xC0 | ch, program & 0x7F, 0], 2));
                }
                _ => {}
            }
            context.send_event(event);
        }

        self.sync_params();
        ProcessStatus::Normal
    }
}

impl ClapPlugin for BassStationPlugin {
    const CLAP_ID: &'static str = "org.novation.bassstation-1994";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Linux CLAP controller for the 1994 Novation BassStation");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Synthesizer,
        ClapFeature::Stereo,
    ];
}

nih_export_clap!(BassStationPlugin);
