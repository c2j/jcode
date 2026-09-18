# Minimal ("tiny") jcode build

This document describes how to build and run the smallest practical jcode that
can still do basic agentic coding, and what that costs.

Scope: this is **step 1** of the minimal-build effort. It changes only the Cargo
profile, an existing feature-profile setting, and runtime config. No Rust code
was restructured, and no subsystem was removed from the source tree. See
[Later steps](#later-steps) for the changes that would go further.

## Quick start

```bash
# 1. Build the small binary (slow: fat LTO, one codegen unit).
scripts/build_tiny.sh

# 2. Apply the matching reduced runtime config.
mkdir -p "${JCODE_HOME:-$HOME/.jcode}"
cp assets/config-templates/tiny.toml "${JCODE_HOME:-$HOME/.jcode}/config.toml"

# 3. Opt out of telemetry (not config-driven).
touch "${JCODE_HOME:-$HOME/.jcode}/no_telemetry"

# 4. Run it.
./target/tiny/jcode --no-update
```

The result lands at `target/tiny/jcode`. It is **not** `target/debug/jcode` or
`target/release/jcode`; those are separate profiles and are left untouched by
`build_tiny.sh`. If you have an old debug build around, it will still be hundreds
of megabytes and is not what the tiny build produced.

If you do not want the reduced runtime config, the build alone is still useful:

```bash
JCODE_DEV_FEATURE_PROFILE=minimal scripts/dev_cargo.sh build \
    --profile tiny -p jcode --bin jcode
```

`scripts/install_release.sh` does not know about the `tiny` profile. If you want
the tiny binary on `PATH` without disturbing the self-dev / stable channels,
install it under its own directory and point a launcher at it yourself:

```bash
install -m 755 target/tiny/jcode "$HOME/.local/bin/jcode-tiny"
```

## What the build changes

### Feature set: `JCODE_DEV_FEATURE_PROFILE=minimal`

`minimal` maps to `--no-default-features` (see `scripts/dev_cargo.sh`), which
drops the root crate's default optional stacks:

| Feature | Dependency stack dropped |
|---|---|
| `embeddings` | `jcode-embedding` -> `tract-onnx`, `tract-hir`, `tokenizers` (onig). **~163 crates.** Removing this also removes the ~87 MB all-MiniLM-L6-v2 ONNX model load. |
| `bedrock` | `jcode-provider-bedrock` -> ~25 `aws-*` crates. |
| `pdf` | `jcode-pdf` text extraction. |

This profile is already exercised by `scripts/test_fast.sh`, so the minimal
feature set is a known-compiling configuration, not a new one.

### Cargo profile: `[profile.tiny]`

Defined in the root `Cargo.toml`. Versus the default `release` profile:

| Setting | `release` | `tiny` | Why |
|---|---|---|---|
| `opt-level` | `1` | `"z"` | Optimize for size. |
| `lto` | off | `"fat"` | Whole-program LTO. The single largest size win. |
| `codegen-units` | `256` | `1` | Lets LTO see everything in one unit. |
| `incremental` | `true` | `false` | Incremental artifacts bloat the binary and block LTO. |
| `strip` | off | `"symbols"` | Drops the symbol table from the shipped binary. |
| `debug` | `0` | `0` | No debug info emitted. |
| `panic` | unwind | `"abort"` | Removes unwind tables. **Behavior tradeoff, see below.** |

The interactive hot-path crates (`ratatui` family, `crossterm`, `unicode-*`,
`jcode-tui-anim`, `jcode-fuzzy`) are pinned back to `opt-level = 3` inside the
profile, matching the existing dev/selfdev/test pins, because at `opt-level = "z"`
the per-frame render loop and fuzzy picker feel visibly laggy.

### Low-memory hosts

Fat LTO builds the whole program in a single LLVM module, so its peak memory is
set by total IR volume and **cannot** be lowered by reducing Cargo's job count:
the final `jcode(bin)` crate is one rustc unit. It peaks at ~2.6 GiB of rustc RSS
(measured on aarch64 with the `minimal` feature set).

If the host has less RAM than that, the build does not fail fast, it thrashes
swap. Measured on a 215 MiB ppc64 host (a PlayStation 3 running Linux), the final
step ran for **~63 hours** before producing a correct binary, with ~2.8 GiB
swapped out and ~87% of its CPU time in the kernel.

`scripts/dev_cargo.sh` checks installed RAM for the fat-LTO profiles and applies:

| Installed RAM | Behavior |
|---|---|
| >= 4096 MiB | keep fat LTO |
| 1024-4096 MiB | keep fat LTO, warn with the measured numbers |
| < 1024 MiB | downgrade to ThinLTO + 256 codegen units |

The downgrade is not free. At `opt-level = "z"` most of the size win is
whole-program dead-code elimination, so ThinLTO gives back a lot of it: on the
aarch64 reference host, fat LTO produces 24.6 MiB and ThinLTO + 16 units produces
42 MiB (**+71%**). That is why the fallback only triggers automatically on hosts
that are too small to run fat LTO at all.

Controls: `JCODE_LOW_MEMORY_LTO=auto|thin|off` (default `auto`),
`JCODE_LTO_MIN_MIB` (default 1024), `JCODE_LTO_WARN_MIB` (default 4096), or an
explicit `CARGO_PROFILE_TINY_LTO=<mode>`. A small host that wants the smallest
binary and is willing to wait can force fat LTO with
`JCODE_LOW_MEMORY_LTO=off`.

### `panic = "abort"` tradeoff

The TUI draw loop, the Mermaid renderer, and PDF extraction use `catch_unwind`
as defense-in-depth so a rendering panic degrades instead of killing the
process. Under `panic = "abort"` those sites still compile but no longer recover:
any panic aborts the process. Session state is persisted server-side, so a
client abort is recoverable by relaunching, but if you want the
graceful-degradation paths back, delete the `panic = "abort"` line from
`[profile.tiny]`.

`panic = "abort"` is also incompatible with `cargo test` for that profile. Test
with the normal profiles; build with `tiny`.

## What the runtime config changes

`assets/config-templates/tiny.toml` turns off background and optional work that
is compiled in but not needed for basic coding:

| Setting | Effect |
|---|---|
| `features.check_updates = false` | Removes the startup update-check thread. Persistent equivalent of `--no-update`. |
| `features.memory = false` | No memory retrieval/extraction, no LLM rerank/judge calls, no embedding model load. |
| `agents.memory_sidecar_enabled = false` | Guarantees no memory sidecar request is attempted. |
| `features.swarm = false` | No swarm workers or inline gallery. |
| `features.mermaid = false` | No Mermaid rendering or Mermaid prompt guidance. |
| `features.auto_poke = false` | No automatic follow-up turns on incomplete todos. |
| `display.performance = "minimal"` | Caps redraw at 12 fps, disables decorative animations, slows side-panel refresh. |
| `display.idle_animation = false`, `prompt_entry_animation = false`, `disabled_animations = [...]` | Removes animation redraw load. |
| `display.latex_rendering = "none"`, `pin_images = false` | Keeps LaTeX and images out of the image pipeline. |
| `power.prevent_sleep_while_streaming = false` | No sleep inhibition. |
| `notifications.turn_complete = false`, `safety.desktop_notifications = false` | No desktop notifications, no macOS notification broker spawn. |
| `autoreview.enabled = false`, `autojudge.enabled = false` | No extra end-of-turn review/judge sessions or LLM calls. |
| `sponsors.enabled = false` | No `discover_tools` tool, no contact with the discovery endpoint. |
| `tools.profile = "minimal"` | Exposes only `bash`, `read`, `write`, `edit`, `multiedit`, `apply_patch`, `patch`, `agentgrep`, `ls`. Shrinks the per-request tool schema. |

Telemetry is not config-driven. Use `JCODE_NO_TELEMETRY=1` (or `DO_NOT_TRACK=1`)
or the `~/.jcode/no_telemetry` marker file.

## Measured result

Measured on an Apple Silicon (aarch64) macOS workstation, Rust stable, `jcode`
at the commit that introduced this doc. The baseline is the existing
`target/release/jcode`, which was built with the **default** feature set
(`pdf`, `embeddings`, `bedrock`) and the `release` profile, so the comparison
covers both the feature-set reduction and the profile change together:

| Metric | `release` (default features) | `tiny` + `minimal` | Change |
|---|---:|---:|---:|
| Binary size | 136,489,952 B (136 MB) | 24,613,488 B (23.5 MB) | **-82%** |
| `__text` (code) | 64,617,564 B | 16,117,640 B | -75% |
| `__eh_frame` | 9,480,336 B | 95,280 B | -99% |
| `__gcc_except_tab` | 5,947,100 B | 9,188 B | -99.8% |

The `__eh_frame` / `__gcc_except_tab` collapse comes from `panic = "abort"`
plus `strip = "symbols"`; the `__text` reduction comes from `opt-level = "z"` +
fat LTO + dropping the embedding/AWS/PDF dependency stacks.

Build time for the tiny profile was ~16 minutes on a 10-core machine, but the
memory-aware job sizing in `scripts/dev_cargo.sh` throttled it to a single rustc
job because the host was under memory pressure; on an unloaded machine it is
faster.

The same fat-LTO `tiny` + `minimal` build on a ppc64 host (PlayStation 3,
215 MiB RAM) produced a **34,662,880 B (34.7 MB)** stripped binary. It is larger
than aarch64 because of the PowerPC ISA, not because the profile did not apply.
The final `jcode(bin)` LTO step took **~63 hours** on that host because it
thrashed swap; see [Low-memory hosts](#low-memory-hosts).

## Measuring the result

```bash
du -h target/tiny/jcode
size -m target/tiny/jcode        # macOS section breakdown
```

At runtime, inside a session:

- `:debug memory` — process memory profile
- `:debug memory-history` — sampled history
- `:debug markdown:memory`, `:debug mermaid:memory` — cache profiles

For a compile-time attribution of what is still large, install `cargo-bloat`
and run it against the built binary.

## What this does *not* shrink

- **The client/server split.** The TUI still needs the long-lived
  `jcode serve` daemon; that process is the main steady-state memory owner.
  Removing it means giving up the interactive TUI.
- **Providers, Mermaid, image, and syntax-highlighting code.** All of these are
  unconditional dependencies today; the `minimal` feature profile does not
  gate them. Trimming them requires the dependency/feature work below.
- **macOS integration helpers** (global hotkey, notification broker,
  `macos_computer_use`).

## Later steps

These are analyzed but deliberately **not** implemented here:

1. **Feature-gate the unconditional stacks** — make the nine
   `jcode-provider-*-runtime` crates optional and gate their
   `register_external_provider*` calls in `src/cli/startup.rs`; give
   `jcode-tui-mermaid` a real feature flag and set
   `default-features = false`; gate `syntect`/`image`/`jcode-terminal-image`;
   gate the macOS-only dependencies.
2. **Compile-time removal of subsystems** — `memory`, `swarm`, side panel, MCP
   pool, and the extra tools are runtime-toggled but always compiled. Gating
   them with `#[cfg]` is the largest remaining win and the largest change.
3. **Trim the tool surface at the registry level** so disabled tools do not
   exist in the binary or the schema at all.
