# Teamy Capability Lab

**Offline experiment.** The original transfer fixture, focused tests, local demo and full `check-all.ps1` quality gate passed on Windows with Rust 1.96.0 at commit `d78960f`. The later identity/disclosure slice has also passed focused tests and the full quality gate. The resolved lockfile records the dependency versions.

The experiment uses fixed, fictional data to compare directional routes for one artifact. It models a desktop, a Dot VM, a Codex environment definition, two distinct running Codex instances, and a store. The definition is not an instance. Endpoint-specific send, receive and processing capabilities, artifact identity, permissions and estimated costs are checked in Rust. Nothing discovers live capabilities, performs transfers, provisions a machine, calls a cloud service or obtains credentials. A plan does not authorize execution.

The fixture has three alternatives: a direct route with unknown permission, a permitted route through the store, and a route through the Dot VM with a denied transfer. Results rank ready routes first, then approval-needed, then blocked; estimated cost and transfer IDs break ties within each category. Approval needs remain visible even for blocked routes.

Scryer Prolog 0.10.0, with default features disabled, enumerates simple paths using fixed rules and generated numeric-only opaque IDs. It is not a sandbox for arbitrary Prolog. A maximum depth, solution count and per-solution inference limit bound search, with caller cancellation checked between query results. Search reports `Complete`, `DepthLimit`, `SolutionLimit`, `InferenceLimit`, or `Cancelled`. Any limit/cancellation result is incomplete: ordering applies only to returned candidates and cannot establish global optimality or absence of a feasible route. Depth below the maximum possible simple-path length is conservatively incomplete. Cyclic paths are intentionally excluded; costs are nonnegative.

Run the CLI:

```powershell
cargo run -- --output-format text plan
cargo run -- --output-format json plan
cargo run -- --output-format json plan --excalidraw .\alternatives.excalidraw

cargo run -- --output-format json disclosure
```

The optional diagram path creates a new local file and refuses to overwrite one. It uses Cloud Terrastodon's public `ExcalidrawScene::from_objects` to show actual returned routes, estimated costs, status and search completeness. Local file errors or cancellation can leave a partial newly created file. The shared Teamy output layer supplies text and JSON; CSV is inherited but nested plans are intended for text or JSON. `--log-file` is another explicit local output option.

An additional synthetic [identity and disclosure fixture](docs/identity-and-disclosure.md) models principal/device distinctions and one Jeff-tagged VM projection. Its focused tests and the full quality gate have passed. It is not an authorization or delegation system.

## Dependencies and boundaries

Cloud Terrastodon registry and Excalidraw crates are pinned to Git revision `fd989bd357cbfc5d60ad06aff2a7677a3a46491f`; no Azure or private UI crate is intentionally selected. The registry integration test defines a local test-only request and checks linked registration and that planning does not invoke it. There are no live transfer executors. The experiment does not use CT's shape-only discovery planner or its one-representative-per-shape simulation as the identity-aware planner.

Teamy CLI uses `teamy-cancellation =0.8.0` and CT uses `=0.7.0`, whose scope types cannot be interchanged. The planner does not call CT's scope-taking structural planner: it checks the CLI's 0.8 scope between bounded Scryer results. The diagram API used is synchronous and does not require a CT cancellation scope. This avoids substituting an unrelated cancellation root, but does not interrupt Scryer within a bounded Prolog segment or interrupt a blocked file write.

On Windows with Rust 1.96, Scryer 0.10.0 aborted in its default interactive top-level initialization: its error path attempted to deallocate a null heap pointer. The experiment uses a minimal embedded top-level, which allowed the focused tests and demo to pass. This avoids the observed startup path but does not establish that every Scryer error path is safe; keep it experimental.

The original template documented a Serde-free dependency tree. Scryer 0.10.0 directly depends on Serde and serde_json even with defaults disabled. The template's `AGENTS.md` does not prohibit transitive Serde; its local lint policy prohibits using Serde APIs in this project's code. The resolved all-features normal dependency tree was checked and contains no Cloud Terrastodon Azure crate.


## Source and license provenance

This project was initialized from [Teamy Rust CLI](https://github.com/TeamDman/teamy-rust-cli), local revision `d3e304a7390b136d5566bcb4599f644c8cb60cc7`. Its package declares MPL-2.0; the unchanged template license is retained in `LICENSE`. The template's `resources/main.png` and `resources/main.ico` are included unchanged. No separate asset-specific license or provenance notice was found for those images in the source checkout; their independent artwork rights have not been verified.

Cloud Terrastodon declares LiLiQ-R-1.1 in its workspace manifest. Its registry and Excalidraw crates are linked dependencies, not copied implementation. The synthetic Excalidraw scene was generated using its scene API and retains the generator's source field; whether the generated scene has separate license obligations has not been independently determined. Scryer Prolog 0.10.0 declares BSD-3-Clause and is also a dependency. `Cargo.lock` identifies the resolved third-party packages; their own licenses apply independently of the project license.

## Run Capability Lab with Relax

From the repository root on Windows x64, run the registry test or either CLI fixture. Quote the argument separator so PowerShell passes it to Relax.

```powershell
.\relax.ps1 test --allow-c teamy-capability-lab --test registry_integration
.\relax.ps1 run --quiet --allow-c teamy-capability-lab '--' --output-format json disclosure
.\relax.ps1 run --quiet --allow-c teamy-capability-lab '--' --output-format json plan
```

The launcher reads the Makepad fork and commit from `[package.metadata.relax]` in [Cargo.toml](Cargo.toml). On first use it fetches that revision and builds the Relax runner under `target/relax-tool`. Later runs reuse the checkout and binary. To change compiler versions, update the full commit SHA in `rev`.

This revision handles the wrapped section attributes LinkMe uses to register entries and the argument files Cargo uses for long Windows compiler commands.

Relax targets the Windows GNU ABI. Install the Rust standard library for that target with `rustup target add x86_64-pc-windows-gnu`. Put the x86-64 LLVM-MinGW `bin` directory on `PATH`, or extract the toolchain so that `target/llvm-mingw/bin` contains `cc.exe` and `windres.exe`. The lab's Scryer Prolog dependency uses `ring`, and `--allow-c` lets Relax build its native code.

## Validation

The full quality gate passed with one Cargo job and one test thread. To reproduce it and the focused checks:

```powershell
$env:CARGO_BUILD_JOBS = "1"
$env:RUST_TEST_THREADS = "1"
cargo test --test registry_integration
cargo tree -i cloud_terrastodon_azure
cargo tree -d
cargo test --test planning
cargo test --test identity_disclosure
cargo test --test disclosure_cli
cargo test --test plan_cli
cargo run -- --output-format json plan
cargo run -- --output-format json plan --excalidraw .\alternatives.excalidraw

cargo run -- --output-format json disclosure
cargo fmt --all -- --check
cargo clippy --all-targets --all-features --no-deps -- -D warnings
cargo build --all-features
./check-all.ps1
```

The all-features normal dependency tree contained no Cloud Terrastodon Azure crate; a `cargo tree -i` query for an absent crate exits with an error. The `demo` directory contains a text report, JSON report, editable Excalidraw scene, and an approximate rendered preview. Focused tests cover direction, instance isolation, missing artifacts and capabilities, permissions, costs, cycles, budget exhaustion, JSON/Excalidraw roundtrip and the registry executor counter.
