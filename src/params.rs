use std::sync::Mutex;
use nih_plug::prelude::*;

pub const CC_MOD_WHEEL: u8 = 1;
pub const CC_VOLUME: u8 = 7;
pub const CC_FILTER_FREQ: u8 = 105;
pub const CC_FILTER_RES: u8 = 106;
pub const CC_FILTER_MOD_DEPTH: u8 = 107;
pub const CC_ENV1_ATTACK: u8 = 108;
pub const CC_ENV1_DECAY: u8 = 109;
pub const CC_ENV2_ATTACK: u8 = 114;
pub const CC_ENV2_DECAY: u8 = 115;
pub const CC_ALL_NOTES_OFF: u8 = 123;

#[derive(Params)]
pub struct BassStationParams {
    #[persist = "midi_out"]
    pub saved_midi_out: Mutex<String>,

    #[persist = "midi_in"]
    pub saved_midi_in: Mutex<String>,

    #[id = "midi_ch"]
    pub midi_channel: IntParam,

    #[id = "mod_wh"]
    pub mod_wheel: IntParam,

    #[id = "vol"]
    pub volume: IntParam,

    #[id = "flt_freq"]
    pub filter_freq: IntParam,

    #[id = "flt_res"]
    pub filter_res: IntParam,

    #[id = "flt_mod"]
    pub filter_mod_depth: IntParam,

    #[id = "env1_atk"]
    pub env1_attack: IntParam,

    #[id = "env1_dec"]
    pub env1_decay: IntParam,

    #[id = "env2_atk"]
    pub env2_attack: IntParam,

    #[id = "env2_dec"]
    pub env2_decay: IntParam,

    #[id = "pb"]
    pub pitch_bend: IntParam,
}

impl Default for BassStationParams {
    fn default() -> Self {
        Self {
            saved_midi_out: Mutex::new(String::new()),
            saved_midi_in: Mutex::new(String::new()),
            midi_channel: IntParam::new("MIDI Channel", 1, IntRange::Linear { min: 1, max: 16 }),
            mod_wheel: IntParam::new("Mod Wheel (CC 1)", 0, IntRange::Linear { min: 0, max: 127 }),
            volume: IntParam::new("Volume (CC 7)", 100, IntRange::Linear { min: 0, max: 127 }),
            filter_freq: IntParam::new("Filter Freq (CC 105)", 64, IntRange::Linear { min: 0, max: 127 }),
            filter_res: IntParam::new("Filter Res (CC 106)", 0, IntRange::Linear { min: 0, max: 127 }),
            filter_mod_depth: IntParam::new("Filter Mod (CC 107)", 64, IntRange::Linear { min: 0, max: 127 }),
            env1_attack: IntParam::new("Env1 Attack (CC 108)", 0, IntRange::Linear { min: 0, max: 127 }),
            env1_decay: IntParam::new("Env1 Decay (CC 109)", 64, IntRange::Linear { min: 0, max: 127 }),
            env2_attack: IntParam::new("Env2 Attack (CC 114)", 0, IntRange::Linear { min: 0, max: 127 }),
            env2_decay: IntParam::new("Env2 Decay (CC 115)", 64, IntRange::Linear { min: 0, max: 127 }),
            pitch_bend: IntParam::new("Pitch Bend", 0, IntRange::Linear { min: -64, max: 63 }),
        }
    }
}
