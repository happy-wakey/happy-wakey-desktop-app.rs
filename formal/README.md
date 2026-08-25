# Formal application-state verification

`app_state.qnt` is the language-neutral control-state contract for every Happy
Wakey client. The current production implementation is the Rust/Qt desktop
machine in `src/app_state.rs`. As of August 2026 the Happy Wakey workspace and
GitHub organization contain no iOS, Android, or other mobile source repository;
future mobile clients must implement this same transition relation and run the
same conformance traces before they can claim compatibility.

## State ownership

The machine is the sole owner of:

- application readiness (`booting`, `ready`);
- authentication (`signed_out`, `authenticating`, `signed_in`, `failed`);
- onboarding (`welcome`, `account`, `backup`, `essentials`, `ready`,
  `complete`); and
- the lifecycle of calendar, weather, stocks, news, onboarding hydration,
  desktop notification, cloud notification, and cloud-reminder-sync effects.

UI code may request an event and render the resulting snapshot. It may not set
control-state flags directly. Editable configuration cannot set onboarding or
authentication state.

Every asynchronous effect that can mutate modeled control state receives a
monotonically increasing token. A result may commit only while its lane is
`running` and its token is still active.
Logout clears every authenticated lane, so a late OAuth, calendar, onboarding,
or cloud callback is explicitly stale and cannot restore or overwrite state.

Every event is total and produces one disposition:

- `applied`: a validated next state was committed atomically;
- `stale`: an obsolete completion intentionally stuttered; or
- `rejected`: an invalid request failed closed without changing state.

## Checked safety properties

`app_state_safety` checks 21 generated verification conditions covering:

1. finite app, auth, onboarding, lane, token, and generation domains;
2. an auth token exists exactly while authentication is active;
3. each running lane has exactly one current token;
4. active tokens equal their lane generation and never exceed the global
   generation;
5. authenticated lanes can run only while signed in;
6. no effect can run before application readiness; and
7. completed onboarding cannot regress during reconciliation.

An irreversible completion-witness bit makes the last property a state
invariant: transition branches can set the witness but cannot clear it, so any
future path away from `complete` is a machine-checkable violation.

The production Rust tests independently explore the reachable graph from every
valid persisted authentication/onboarding startup combination, through two
global operation generations across all eight lanes. For every event from every
visited state they check totality, determinism, post-transition invariants,
stale-result suppression, independent-lane behavior, strict onboarding edges,
and reachability of every auth/onboarding/lane phase.

## Reproducible verification

The tool versions are pinned because the model and checker are part of the
proof input.

```bash
QUINT_PACKAGE='@informalsystems/quint@0.32.0'
JAVA_HOME='/opt/homebrew/opt/openjdk@21/libexec/openjdk.jdk/Contents/Home'

npx --yes --package="$QUINT_PACKAGE" quint typecheck formal/app_state.qnt
npx --yes --package="$QUINT_PACKAGE" quint typecheck formal/app_state_test.qnt
npx --yes --package="$QUINT_PACKAGE" quint test \
  formal/app_state_test.qnt --main=app_state_test --match='.*Test$'
npx --yes --package="$QUINT_PACKAGE" quint run \
  formal/app_state.qnt --main=app_state \
  --max-samples=10000 --max-steps=24 \
  --invariant=app_state_safety
PATH="$JAVA_HOME/bin:$PATH" npx --yes --package="$QUINT_PACKAGE" quint verify \
  formal/app_state.qnt --main=app_state \
  --max-steps=4 --invariant=app_state_safety
cargo test --locked app_state
```

The local verification baseline is:

- ten deterministic Quint traces passed;
- 10,000 randomized 24-step traces passed, witnessing every app, auth, lane,
  and onboarding terminal phase;
- Apalache found no violation in any execution through four transitions; and
- the exhaustive production-state explorer passed.

## Mobile conformance gate

An iOS, Android, Flutter, Kotlin Multiplatform, or other client is conformant
only when it:

1. uses one private transition kernel with the same states and events;
2. implements `applied`, `stale`, and `rejected` without exceptions or implicit
   fallback transitions;
3. uses monotonic operation tokens and invalidates authenticated lanes on
   logout;
4. derives UI capabilities and loading indicators from the machine snapshot;
5. passes the ten traces in `app_state_test.qnt` as native tests;
6. exhaustively explores its bounded native transition graph; and
7. records the SHA-256 of its implementation and both Quint files in CI.

Do not copy mutable UI booleans into a mobile implementation. Port the pure
transition function and keep platform effects outside it.

## Proof boundary

This is a safety proof for the declared finite abstraction and checked bounds,
plus exhaustive testing of the bounded production graph. It proves that
declared control-state transitions cannot produce an invalid modeled state. It
does not prove that operating systems, OAuth providers, networks, notification
services, filesystems, clocks, native Qt code, or hardware never fail. Those
systems are environmental inputs. Their completions must return through the
token-checked machine and resolve to a controlled success, failure, rejection,
or stale stutter. Best-effort Supabase mirror writes report status but do not
mutate modeled control state; remote service ordering and durability remain
outside this proof boundary.
