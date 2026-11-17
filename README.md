# bassstation-clap

A Linux **CLAP** plugin for controlling and automating the **1994 Novation BassStation** (Keyboard, MIDI Implementation `v1.4`) via MIDI CCs, Program Changes, and SysEx (designed for Bitwig Studio and other CLAP hosts).

## Features
* **Filter & Output:**
  * Live **Low-Pass Filter** graph with shaded **Mod Depth (`CC 107`)** sweep range
  * **Filter Frequency (`CC 105`)**, **Resonance (`CC 106`)**, **Mod Depth (`CC 107`)**, **Mod Wheel (`CC 01`)**, and **Volume (`CC 07`)**
* **Envelope 1 & Envelope 2 (Filter):**
  * Live Attack/Decay graphs for **Env 1 (`CC 108`, `CC 109`)** and **Env 2 (`CC 114`, `CC 115`)**
* **Bi-Directional MIDI Sync:** Twisting knobs on the BassStation updates the plugin sliders in real time without echo loops
* **Patch Selector (`P1–P7`):** Sends MIDI Program Changes `0–6`
* **SysEx & Backup (`.syx`):** Capture SysEx dumps from the BassStation and save/reload `.syx` files in `~/BassStation_Patches/`
* **Direct ALSA Sequencer MIDI I/O:** Forwards Bitwig Notes, CCs, Pitch Bend, and Program Changes

## Building on Linux

### Prerequisites
* Rust toolchain (`cargo`, `rustc`)
* ALSA development headers (`libasound2-dev` on Debian/Ubuntu) and `pkg-config`

### Build & Install
```bash
cargo xtask bundle bassstation-clap --release
cp -r target/bundled/bassstation-clap.clap ~/.clap/
```

## License
GPL-3.0-or-later
