# Nusick

Nusick is an MIT-licensed Rust framework for building audio plugins. The
first plugin is **Infinati Kick**, a port of Infinati's JUCE kick synth.

Today [nih-plug](https://github.com/robbert-vdh/nih-plug) is the host layer
(CLAP, standalone, parameters, GUI hosting). Nusick owns the DSP, the UI
look, and the plugins, so the host layer can be swapped for Nusick's own later.

## Layout

| Path | What it is |
|---|---|
| `crates/nusick-dsp` | Pure-Rust DSP with no plugin dependencies: sine oscillator, envelopes, one-pole filter, tanh drive, noise burst, delay line, parameter smoother, and the kick voice. Includes the `render-kick` CLI. |
| `crates/nusick-ui` | Dark egui theme plus `Knob` and `Meter` widgets. |
| `plugins/infinati-kick` | The Infinati Kick plugin (CLAP plus standalone, opt-in VST3). |
| `xtask` | `cargo xtask bundle …`, which builds and lays out the plugin bundles. |
| `scripts/package.sh` | Zips the bundles for release. |

## Infinati Kick

Any MIDI note triggers the kick, and a new note restarts it. A sine wave
sweeps exponentially from **Start Pitch** to **End Pitch**.

| Parameter | Range | Default |
|---|---|---|
| Start Pitch | 40–1000 Hz | 200 Hz |
| End Pitch | 20–200 Hz | 50 Hz |
| Pitch Decay (time for the sweep to fall 60 dB) | 1–500 ms | 60 ms |
| Amp Decay (time for the level to fall 60 dB) | 10–2000 ms | 450 ms |
| Click (noise burst) | 0–100 % | 20 % |
| Drive (tanh) | 0–100 % | 20 % |
| Level (output peak) | -36–0 dB | -3 dB |

All parameters can be automated and are smoothed for about 20 ms to avoid
zipper noise. Drive is gain-compensated, so the output peak always sits at
**Level**.

Knobs: drag up and down to change, hold Shift for fine control, double-click to reset.

## Building

You need the stable Rust toolchain from [rustup.rs](https://rustup.rs).

### macOS

```sh
xcode-select --install                     # once, for the C toolchain
cargo test --workspace
cargo xtask bundle infinati_kick --release
# Optional universal (Intel + Apple Silicon) build:
rustup target add x86_64-apple-darwin aarch64-apple-darwin
cargo xtask bundle-universal infinati_kick --release
```

Install by copying `target/bundled/Infinati Kick.clap` to
`~/Library/Audio/Plug-Ins/CLAP/`. The standalone app is
`target/bundled/Infinati Kick.app`. Release builds are unsigned, so after
downloading run `xattr -dr com.apple.quarantine "Infinati Kick.clap"`.

### Windows

Install the [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
with the "Desktop development with C++" workload, then:

```powershell
cargo test --workspace
cargo xtask bundle infinati_kick --release
```

Copy `target\bundled\Infinati Kick.clap` to `C:\Program Files\Common Files\CLAP\`.
The standalone app is `target\bundled\Infinati Kick.exe`.

### Linux

```sh
sudo apt install libasound2-dev libgl-dev libjack-jackd2-dev libx11-xcb-dev \
    libxcursor-dev libxcb-icccm4-dev libxcb-dri2-0-dev libxcb1-dev
cargo test --workspace
cargo xtask bundle infinati_kick --release   # copy the .clap to ~/.clap/
```

### Quick listening without a DAW

```sh
cargo run -p nusick-dsp --bin render-kick -- kick.wav --start 220 --end 45 --drive 0.4
```

Flags: `--start --end --pitch-decay --amp-decay --click --drive --level --length --rate`.

The standalone app also works: `"target/bundled/Infinati Kick" --help`
lists the audio and MIDI backends.

## Licensing and VST3

Nusick's code is **MIT**. The default build (CLAP plus standalone) contains
**no GPL code**. CI enforces this with `cargo deny check licenses`, and
`NOTICE` lists every license involved.

**VST3 is opt-in and makes the binary GPLv3.** nih-plug's VST3 wrapper uses
`vst3-sys`, which is GPLv3. Steinberg moved the VST3 SDK itself to MIT, but
that doesn't relicense `vst3-sys`. So:

```sh
cargo xtask bundle infinati_kick --release --features vst3-gpl
```

builds a `.vst3` that you can use privately without restriction. **If you
distribute it, you must do so under GPLv3, with source.** Official releases
never include it.

**Why this matters: Ableton Live 12 loads VST3 and AU, not CLAP.** Until
Nusick has its own MIT VST3 wrapper (planned, built on the MIT/Apache `vst3`
crate), use the `vst3-gpl` build for your own sessions in Live.

## Releases

Every push to `main` builds and tests on macOS, Windows and Linux. CI then
replaces the GitHub Release tagged **`latest`** with fresh zips: a universal
macOS build, Windows x64 and Linux x64.

## License

MIT. See [LICENSE](LICENSE) and [NOTICE](NOTICE).
