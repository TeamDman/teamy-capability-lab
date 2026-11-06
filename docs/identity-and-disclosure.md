# Synthetic identity and disclosure slice

This is a tested, in-memory follow-up to the route experiment. It does not authenticate, enforce access, discover resources, execute a VM operation or define external hidden-resource behavior. The existing transfer planner is unchanged. Its focused tests and full `check-all.ps1` quality gate passed with one Cargo job and one test thread.

## Fixture assumptions

`src/capability/identity` distinguishes a principal ID from a desktop or phone device ID and from a concrete environment instance linked to its definition. The fixed session list is synthetic input, not proof of authentication. Alice has a desktop and a phone; Bob has a different phone; an agent has an explicitly identified Codex instance. A requested principal/origin pair absent from that list is unknown. No principal inherits authority merely from using a device, a definition or an instance.

The fixture assesses a request for **one already-selected VM**, rather than listing VMs. For each principal and VM, it uses independent synthetic observations for permission to invoke the one operation and permission for its read effect. A missing observation, an explicitly unknown value or duplicate observations remains unknown; a denial blocks this fixture assessment while the report retains each individual decision, including unknowns. This is a deliberately small example, not a general conflict-resolution policy or a delegated grant. There is no delegation, transitivity, revocation, expiry or approval mechanism.

For a request with both fixture permissions granted, an exact `owner = Jeff` tag selects the synthetic VM. The output type exposes only `powered_on`; a missing power state remains `None`. An unknown owner tag remains unknown. A known different tag produces the internal `NotApplicable` result. That result exists for fixture testing: it does not prescribe whether a real service should disclose that a resource exists, is denied or did not match. The owner tag, OS version and disk label are not included in the projected output. The scoped recipient need not be Jeff.

`ReadPowerRequest` and `ReadPowerOutput` are registered through Cloud Terrastodon's Linkme registry. The assessment checks that this request's declared effect is `Read`, then consults the separate fixture observations. Registration and declared effects are metadata, not permission or proof of safety. The request's `IntoFuture` is a metadata-only stub that records accidental invocation and returns an error; it does not access a VM. Assessment does not invoke the registered executor.

The `disclosure` CLI command evaluates the fixed Alice/desktop/Jeff fixture and emits the projection through the existing text/JSON output layer.

## Test coverage

The focused tests check:

- The same principal's desktop and phone receive the same principal-scoped fixture decision, while another principal and an unrecognized principal/device pairing do not inherit it.
- An explicitly represented agent environment with an unknown read-effect decision remains unknown; changing the concrete instance does not inherit the session.
- Jeff-tagged, different-owner, unknown-owner and unknown-power records have distinct fixture outcomes, and serialized visible output excludes owner, OS and disk fields.
- Unknown or denied invocation/read observations and duplicate facts do not turn into grants.
- The same typed request and output are linked in the registry with the declared read effect, and assessment leaves the executor invocation count unchanged.
- The CLI returns only the synthetic projected property.

## Disclosure boundary review

The fixed `disclosure` CLI selects the synthetic Alice/desktop session and `jeff-on` internally. Its successful JSON projection contains the powered-on property but not the request's principal, device or VM IDs, owner tag, OS version or disk label. Tests check those exclusions. The fixture structs and `assess` function are public in-memory Rust APIs, however, and the fixture includes all synthetic VM fields. The internal assessment distinguishes `NotApplicable`, missing or duplicate records, unknown owner tags, absent or conflicting permission observations, and individual denied/unknown decisions. If an application exposed those facts or error distinctions to an untrusted caller, it could reveal resource existence, attributes or policy configuration. The registry describes the request's session and VM ID shapes; it is not a redaction layer.

No real visibility policy has been chosen. In particular, this experiment makes no claim that hidden and nonexistent resources are indistinguishable. Do not use the assessment, fixture or metadata as an enforcement boundary. Choosing external disclosure and error semantics requires a separate policy and threat model.

## Decisions still open

- What is the identity of a request: a human principal, an agent, a delegated chain, or separate issuer and acting principal? How are sessions bound to endpoints?
- Are effects merely declared metadata, independently authorized requirements, or both? What scope and evidence does each decision require?
- What are the exact semantics of a delegated grant, including attenuation, expiration, revocation, transitivity and approval? Should unknown be shown as approval-needed when approval may not be possible?
- Should filtered or hidden resources be indistinguishable from nonexistent ones to the caller, and how should missing, denied and unknown properties be presented without leaking them?
- Which registry metadata is sufficient to associate a typed operation with effects and policy without treating its declared metadata as proof? `Read`, `Write` and `Solicit` do not encode the policies above.

## Cancellation, registry and provenance boundaries

The existing planning loop checks the Teamy cancellation 0.8 scope through a callback before machine startup and between Scryer answers; the bounded Prolog segment and machine initialization are not preempted. The CLI subsequently returns the original cancellation error instead of emitting a partial cancelled report. Diagram creation and file writes have checkpoints but a blocked write is not interruptible. The new disclosure command checks that same caller scope before its bounded in-memory assessment. Cloud Terrastodon's registry uses cancellation 0.7, but this fixture only inspects metadata and does not call its scope-taking discovery APIs. It must not create an unrelated cancellation root.

The current Scryer minimal-top-level workaround and third-party provenance caveats in the README still apply. The local template declares MPL-2.0, Cloud Terrastodon declares LiLiQ-R-1.1, and Scryer declares BSD-3-Clause. The source checkout provides no separate icon-artwork notice, and the effect of the library license on a generated scene has not been established. No publication is authorized.
