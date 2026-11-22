# Nusick security review

**Scope:** `main` at `d1a1887` (nusick-dsp, nusick-ui, the Kick Me plugin, xtask, CI and release). Reviewed against the policy in AudioZero's `docs/SECURITY_REVIEW.md` and the hardening in AudioZero's Rust code (`Project::sanitize`, the `finite()` helper, bounded decoding, `unsafe_code = "forbid"`, `permissions: contents: read`, checksummed releases). The goal is that the two projects hold the same line, because AudioZero compiles `nusick-dsp` into its own audio thread (it pins this repo by commit) and Nusick's plugins load into the same kinds of hosts.

Everything below was run, not just read: the new tests failed on the old code and pass on the new, `cargo clippy -D warnings`, `cargo test --workspace`, the real `cargo xtask bundle kick_me --release`, and `cargo deny check advisories bans licenses sources` against the real `Cargo.lock`. AudioZero's 229 engine tests were also run against the patched `nusick-dsp` (see "Compatibility with AudioZero").

## Threat model

Nusick is a framework, so its attack surface is smaller than AudioZero's: there is no project loader and no file parser. What reaches it is:

- **Values from a host.** A DAW session stores plugin state, and a session is a file people pass around ("here's my session"). Parameter values, the sample rate and block sizes all arrive from that file or from the host.
- **Values from a caller.** AudioZero (and any other consumer of `nusick-dsp`) passes parameters and a sample rate into the DSP. The DSP is the last line of defence for every consumer, so it must not rely on the caller having validated.
- **Command-line arguments** to `render-kick` and the standalone app.
- **The build and release pipeline**, which produces unsigned binaries people download and run.

The aim is the same as AudioZero's: a hostile value may produce silence or a clamped value, but never a panic on the audio thread, an unbounded allocation, NaN/inf at the output, or output louder than the documented range.

Third-party code (nih-plug and its dependencies) is trusted to the extent any dependency is; section 7 covers how that trust is checked.

## Findings

| # | Severity | Finding | Status |
| --- | --- | --- | --- |
| 1 | **High** | A NaN or inf poisons stateful DSP (oscillator, smoother, filter, drive, envelopes) permanently | Fixed |
| 2 | **High** | A sample rate below about 2 Hz, or NaN, panics on the audio thread | Fixed |
| 3 | Medium | Unbounded allocation (render length and rate, `DelayLine::new`) and WAV header integer overflow | Fixed |
| 4 | Medium | Unbounded magnitudes: `level_db` has no ceiling, other parameters are unclamped | Fixed |
| 5 | Medium | A saved session can set the editor window to any size | Fixed |
| 6 | Low | `render-kick` accepts `nan`/`inf` and casts rates with `as` | Fixed |
| 7 | Low | Supply chain: CI checked licenses only; a RUSTSEC advisory was never seen; nothing enforced the lockfile | Fixed, one pin remains |
| 8 | Low | Release integrity: no checksums, no token permissions, unsigned builds | Partly fixed |
| 9 | Info | Smaller robustness notes | See below |

### 1. NaN and inf poison the DSP for good (High)

This is AudioZero's finding 5 ("NaN passes validation") in a different place. `f32::clamp` passes NaN straight through, and every processor here is a recursive filter of some kind, so one NaN is never forgotten:

- `Smoother::set_target(NaN)` made `current` NaN, and `current = target + (current - target) * coef` stays NaN for any later target.
- `SineOsc::next(NaN, _)` made the phase NaN; `phase -= phase.floor()` keeps it NaN. The oscillator was silent (NaN) until the voice was rebuilt.
- `OnePole` kept a NaN state, `Drive::new(NaN)` gave a NaN gain, and `AdEnvelope::set_times(NaN, ..)` gave a NaN attack step that kept the envelope attacking forever.
- `KickVoice::set_params` and the plugin's per-block `set_params` fed host values into all of these.

The consequence in a host is a kick that goes silent or sends NaN to the audio driver until the plugin is reloaded; in AudioZero it would be NaN reaching the master bus.

**Fix:**

- `nusick_dsp::finite(value, lo, hi, default)`, the equivalent of AudioZero's `finite()`: clamps, never panics, never passes NaN.
- Every processor ignores or replaces a non-finite input and **recovers** on the next sane one (`Smoother`, `SineOsc`, `OnePole`, `Drive`, the envelopes, `DelayLine`).
- `KickParams::sanitized()` clamps every field, and `KickVoice::new` / `set_params` call it, so no caller can bypass it.
- `KickVoice::next` replaces a non-finite sample with 0 as a last line of defence, as AudioZero scrubs before its master clipper.

Tests: `bad_params_never_make_a_non_finite_or_loud_output` (every parameter set to each of nine hostile values, then checked for recovery) and `processors_recover_from_non_finite_input`.

### 2. A bad sample rate panics on the audio thread (High)

`OnePole::set_cutoff` called `cutoff.clamp(1.0, sample_rate * 0.49)`. `f32::clamp` **panics** when `min > max`, which happens for any sample rate under 2.04 Hz, and returns NaN for a NaN rate. `KickVoice::new` builds a `OnePole`, and a host controls the rate. A panic cannot unwind out of a plugin's `extern "C"` entry point, so in practice it aborts the whole DAW process and everything else open in it.

AudioZero already guards its own entry points (`prepare` falls back when the rate isn't finite and positive; the engine accepts 8 kHz to 768 kHz), but a library shouldn't rely on that.

**Fix:** `sane_sample_rate()` accepts 8,000 to 768,000 Hz (the same range as AudioZero's engine) and otherwise returns 48 kHz. It is applied in `KickVoice::new`, `decay_coef`, `OnePole::set_cutoff` and `AdEnvelope::set_times`. `finite()` uses `max`/`min` rather than `clamp`, so it cannot panic whatever the bounds.

Test: `bad_sample_rates_neither_panic_nor_poison` (all hostile values plus 0.5, 1, 2 and 100 Hz).

### 3. Unbounded allocation and WAV header overflow (Medium)

AudioZero's finding 3, for the offline paths:

- `kick::render(params, rate, seconds)` allocated `(seconds * rate).round() as usize` samples. `render-kick --length 1e9` aborted with `capacity overflow`; `--length 1e5 --rate 4e9` would have asked for far more memory than any machine has.
- `DelayLine::new(usize::MAX)` panicked, and a large-but-valid size allocated that much.
- `write_wav_f32` computed `samples.len() * 4` and `sample_rate * 4` with unchecked arithmetic: a panic in debug builds and a silently corrupt header (truncated size) in release builds.

**Fix:** `render` clamps `seconds` to `kick::MAX_RENDER_SECONDS` (60) and uses the sanitised rate; `DelayLine::new` is capped at `DelayLine::MAX_SAMPLES` (16M samples, 64 MB); `write_wav_f32` uses checked arithmetic and returns `InvalidInput` for a zero rate or anything that doesn't fit in the 32-bit header fields.

Tests: `render_is_bounded`, `delay_line_is_bounded_and_finite`, `wav_header_sizes_are_checked`.

### 4. Unbounded magnitudes (Medium)

AudioZero's finding 6. `db_to_gain(level_db)` had no ceiling and is applied *after* the ±1 clamp, so `level_db = 200` produced output 10^10 times full scale, and `+inf` produced `inf` (the new `level_is_capped` test failed on the old code for exactly this); a NaN level gave NaN. Frequencies and decay times were also unclamped.

**Fix:** the documented parameter ranges are now constants on `KickParams` (`START_HZ`, `END_HZ`, `PITCH_DECAY_MS`, `AMP_DECAY_MS`, `CLICK`, `DRIVE`, `LEVEL_DB`), and the DSP clamps to them. **`LEVEL_DB` tops out at 0 dB**, so a bad value can never push the output above full scale, which matters for the same headphone and speaker reasons AudioZero gives. The plugin builds its parameters from the same constants, so the DSP limits and the plugin's ranges cannot drift apart (AudioZero's "one validation layer" principle). `decay_coef` also caps times at `MAX_TIME_MS` (10 minutes).

Test: `level_is_capped`.

### 5. The editor size is restored from the session (Medium)

The Kick Me parameters had `#[persist = "editor-state"] editor_state: Arc<EguiState>`. nih-plug deserialises that field straight from the saved session, and `EguiState` holds an unbounded `(u32, u32)` window size that is passed to the window creation code. A session file could set a 4-billion-pixel window. The editor is fixed-layout and not resizable, so persisting the size had no benefit.

**Fix:** the field is no longer `#[persist]`ed. A session's old `editor-state` entry is simply ignored. (Parameter IDs are unchanged, so no saved session breaks.)

### 6. `render-kick` argument handling (Low)

`"nan"` and `"inf"` parse as `f32`, so `--start nan` was accepted, and `--rate` went through `v as u32`, which saturates silently. **Fix:** non-finite values are rejected, out-of-range settings are clamped with a visible note, and `--rate` / `--length` are range-checked with a clear error. Like AudioZero's `--screenshot`, the tool writes to whatever path it is given; that's fine for a developer CLI.

### 7. Supply chain (Low)

- **Advisories were never checked.** `cargo deny check licenses sources` ran in CI, but not `advisories`. Running it found **RUSTSEC-2026-0274** (`rtrb` 0.2.3, double free when an element's `Drop` panics in `ReadChunk::commit`). nih-plug is the only user, in its standalone backend, and pins `rtrb ^0.2.2`, so it can't be upgraded from here. It is not reachable: nih-plug only calls `push`/`pop` on `f32` and `NoteEvent`, never `read_chunk`/`commit`, and neither type has a `Drop`. It is recorded as an ignore in `deny.toml` **with that reasoning and the condition for removing it** (nih-plug moving to `rtrb >= 0.3.5`). AudioZero uses `rtrb 0.4.1` and is not affected.
- **CI now runs `check advisories bans licenses sources`**, plus a weekly scheduled run, because advisories appear without any change to the repository. `yanked = "deny"`, `wildcards = "deny"` and `unknown-registry = "deny"` are set. The workspace crates are marked `publish = false`, which is also true in practice and stops an accidental `cargo publish`.
- **The lockfile is enforced.** CI runs `cargo fetch --locked` and uses `--locked` on clippy, test and run, as AudioZero's `rust.yml` does.
- **`unsafe_code = "forbid"`** is now a workspace lint on every crate, matching AudioZero. There was no `unsafe` in the repository, so this prevents regressions. It also compiles cleanly for the plugin crate, because lints don't fire inside nih-plug's export macros.
- **Dependabot** (`.github/dependabot.yml`) bumps CI actions and Cargo dependencies weekly.
- **Already good:** nih-plug is pinned to a full commit hash (better than AudioZero's JUCE pin by tag), `unknown-git = "deny"` with an explicit allow-list, and `default-features = false` keeps the GPL `vst3-sys` out of the default build.
- **Remaining:** third-party actions (`actions/*`, `dtolnay/rust-toolchain`, `Swatinem/rust-cache`, `EmbarkStudios/cargo-deny-action`) are still pinned by tag, as AudioZero's are. Pinning them to commit SHAs needs the SHAs looked up and checked; Dependabot will then keep them current. I did not guess at hashes.

### 8. Release integrity (Low)

- **Token permissions:** the workflow had no `permissions:` block, so build jobs got the repository default (possibly write). It now has `permissions: contents: read` at the top, and only the `release` job asks for `contents: write`, as in AudioZero's `rust.yml`. There is no `pull_request_target`.
- **Checksums:** the release job now publishes `SHA256SUMS.txt` with the zips, as AudioZero's does.
- **Artefact retention:** CI artefacts keep for 14 days.
- **Remaining, not fixable in the repository:** builds are **unsigned** (macOS Developer ID and notarisation, Windows Authenticode), as AudioZero's are. Consider `actions/attest-build-provenance` once signing is sorted.

### 9. Info

- `gh release delete latest ... || true` followed by `create` leaves a short window with no release, and a failed create leaves none. Editing the release in place, or creating before deleting, would avoid that. It isn't a security issue.
- `.cargo/config.toml` makes `cargo xtask` run `--release`; fine, but it means the bundler is built from the same lockfile and checked by `cargo fetch --locked` above.
- **No fuzz target was added.** The DSP has no parser; `hostile_input.rs` covers the value space deterministically. If a state or preset format is ever added to Nusick, it should get the `Sanitize`-then-use pattern and a fuzz target like AudioZero's plan.
- **Out-of-process hosting** (AudioZero's finding 8) doesn't apply: Nusick builds plugins, it doesn't host them.

## Compatibility with AudioZero

- The public API of `nusick-dsp` only **grew**: new constants (`MIN_SAMPLE_RATE`, `MAX_SAMPLE_RATE`, `DEFAULT_SAMPLE_RATE`, `MAX_TIME_MS`, `KickParams::*`, `kick::MAX_RENDER_SECONDS`, `DelayLine::MAX_SAMPLES`) and functions (`finite`, `sane_sample_rate`, `KickParams::sanitized`). `KickVoice::new` and `KickVoice::set_params` keep their signatures, and `write_wav_f32` still returns `io::Result`.
- Behaviour only changes for values that were already invalid. Valid parameters render **byte-for-byte as before**: `render-kick` output for the defaults and for a custom kick was compared between the old and new code with `cmp` and is identical.
- AudioZero's `audiozero-engine` test suite (229 tests) passes against this `nusick-dsp` using a temporary `--config patch` (AudioZero's files were not modified).
- **To pick this up in AudioZero**, bump the `rev` of the `nusick-dsp` git dependency in `crates/audiozero-engine/Cargo.toml` (and `Cargo.lock`) to the commit that contains this change, once it is on `main`.
- Both repositories now share the same policy: `unsafe_code = "forbid"`, clippy `all = warn`, a `finite()` helper with the same semantics, the same sample-rate range (8 kHz to 768 kHz), `permissions: contents: read`, `--locked` builds and checksummed releases. Nusick additionally enforces cargo-deny advisories, bans and sources, which AudioZero does not yet (it has no `deny.toml`); that is worth copying back to AudioZero.
