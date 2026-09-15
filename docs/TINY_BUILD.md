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

## Building on a resource-constrained host

The `tiny` profile is the **most memory-hungry** build this repo offers: fat LTO
with a single codegen unit forces the final `jcode` binary's link step to hold
the whole program at once, and the largest single rustc unit (`jcode-base`)
peaks around 1.6 GiB RSS on its own. Budget accordingly before starting on a
small machine.

### 0. Check what the build wrapper will do first

Always build through `scripts/dev_cargo.sh` (which `scripts/build_tiny.sh` uses)
rather than bare `cargo`. It sizes the job count from *currently available*
memory, picks a fast linker, and takes a build lock so parallel builds on the
same host do not stampede into OOM:

```bash
scripts/dev_cargo.sh --print-setup
```

`build_jobs_status` shows the chosen job count and the available-memory figure it
was derived from. The default budget is 1792 MiB per job; override it with
`JCODE_BUILD_MIB_PER_JOB` if your machine is tighter.

### 1. On-device build

```bash
export JCODE_DEV_FEATURE_PROFILE=minimal   # drop ONNX/AWS/PDF dependency stacks
export JCODE_BUILD_JOBS=1                  # force serial rustc if in doubt
scripts/build_tiny.sh
```

Also make sure there is **swap** (a `zram` device or a swapfile on Linux) and
enough disk: the `minimal` feature set still writes ~1.6 GB into `target/tiny`.
Reclaim space with `scripts/clean_target.sh` (dry-run by default; `--apply`
removes regenerable cross-compile caches).

### 2. If the fat-LTO link OOMs

The profile settings are plain Cargo config, so they can be overridden from the
environment without editing `Cargo.toml`:

```bash
# Thin LTO and more codegen units: much lower peak memory, somewhat larger binary.
CARGO_PROFILE_TINY_LTO=thin CARGO_PROFILE_TINY_CODEGEN_UNITS=16 scripts/build_tiny.sh
```

Or step all the way back to an existing profile and keep only the reduced
feature set, which is where most of the size win actually comes from:

```bash
JCODE_TINY_PROFILE=release-lto scripts/build_tiny.sh   # thin LTO
JCODE_TINY_PROFILE=release     scripts/build_tiny.sh   # no LTO, fastest build
```

### 3. Or build somewhere else and copy the binary back

If the device cannot host the build at all, offload it. `scripts/remote_build.sh`
rsyncs the tree to a remote SSH host, builds there, and syncs the binary back;
it understands `--profile <name>`, so it works with `tiny`:

```bash
mkdir -p ~/.config/jcode
cat > ~/.config/jcode/remote-build.env <<'EOF'
JCODE_REMOTE_HOST=mybuilder
EOF

JCODE_DEV_FEATURE_PROFILE=minimal JCODE_BUILD_JOBS=1 \
    scripts/remote_build.sh --profile tiny -p jcode --bin jcode
```

`dev_cargo.sh` can also do this transparently: set `JCODE_REMOTE_CARGO=1` (in the
same env file or the shell) and every build, including
`scripts/build_tiny.sh`, is offloaded, falling back to local cargo when the host
is unreachable.

The remote host must be the **same platform** as the device, since the artifact
is copied over and executed directly. For a different platform, cross-compile
instead (see issue #1078 for the `zigbuild`-style Raspberry Pi case).

### 4. Repeat builds

Non-incremental profiles are the ones sccache can actually accelerate:
`scripts/build_tiny.sh` runs with `incremental = false`, so the wrapper enables
sccache when it is available. Keep it on for iterative work on a slow host, and
reuse the same `target/` directory between runs.

### Realistic expectations

~16 minutes on a 10-core Apple Silicon host *throttled to a single job* by memory
pressure. A 4-core single-board computer is in the hours range, and the link step
is the part most likely to fail first.

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
