// Bounded, total model of the Happy Wakey cross-platform control state.
//
// Unsupported state/event pairs stutter. Async results commit only when their
// token equals the active generation for the corresponding auth/resource lane.
module app_state {
  type State = {
    app_phase: int,
    auth_phase: int,
    auth_generation: int,
    auth_token: int,
    onboarding: int,
    onboarding_completed_once: bool,
    generation: int,
    lane_phase: int -> int,
    lane_generation: int -> int,
    lane_token: int -> int,
  }

  pure val BOOTING = 0
  pure val READY = 1

  pure val SIGNED_OUT = 0
  pure val AUTHENTICATING = 1
  pure val SIGNED_IN = 2
  pure val AUTH_FAILED = 3

  pure val WELCOME = 0
  pure val ACCOUNT = 1
  pure val BACKUP = 2
  pure val ESSENTIALS = 3
  pure val ONBOARDING_READY = 4
  pure val COMPLETE = 5

  pure val IDLE = 0
  pure val RUNNING = 1
  pure val LANE_READY = 2
  pure val LANE_FAILED = 3

  pure val CALENDAR = 0
  pure val WEATHER = 1
  pure val STOCKS = 2
  pure val NEWS = 3
  pure val ONBOARDING_HYDRATION = 4
  pure val DESKTOP_NOTIFICATION = 5
  pure val CLOUD_NOTIFICATION = 6
  pure val CLOUD_REMINDER_SYNC = 7
  pure val BLUETOOTH = 8

  pure val STARTUP_COMPLETED = 0
  pure val LOGIN_REQUESTED = 1
  pure val LOGIN_SUCCEEDED = 2
  pure val LOGIN_FAILED = 3
  pure val LOGOUT_REQUESTED = 4
  pure val LANE_REQUESTED = 5
  pure val LANE_SUCCEEDED = 6
  pure val LANE_FAILED_EVENT = 7
  pure val ONBOARDING_NEXT = 8
  pure val ONBOARDING_PREVIOUS = 9
  pure val ONBOARDING_SKIP = 10
  pure val ONBOARDING_FINISH = 11
  pure val ONBOARDING_RECONCILED = 12

  pure val MAX_GENERATION = 3
  pure val APP_PHASES = Set(BOOTING, READY)
  pure val AUTH_PHASES = Set(SIGNED_OUT, AUTHENTICATING, SIGNED_IN, AUTH_FAILED)
  pure val ONBOARDING_STEPS = Set(
    WELCOME,
    ACCOUNT,
    BACKUP,
    ESSENTIALS,
    ONBOARDING_READY,
    COMPLETE
  )
  pure val LANE_PHASES = Set(IDLE, RUNNING, LANE_READY, LANE_FAILED)
  pure val LANES = Set(
    CALENDAR,
    WEATHER,
    STOCKS,
    NEWS,
    ONBOARDING_HYDRATION,
    DESKTOP_NOTIFICATION,
    CLOUD_NOTIFICATION,
    CLOUD_REMINDER_SYNC,
    BLUETOOTH
  )
  pure val AUTH_LANES = Set(
    CALENDAR,
    ONBOARDING_HYDRATION,
    CLOUD_NOTIFICATION,
    CLOUD_REMINDER_SYNC
  )
  pure val EVENTS = Set(
    STARTUP_COMPLETED,
    LOGIN_REQUESTED,
    LOGIN_SUCCEEDED,
    LOGIN_FAILED,
    LOGOUT_REQUESTED,
    LANE_REQUESTED,
    LANE_SUCCEEDED,
    LANE_FAILED_EVENT,
    ONBOARDING_NEXT,
    ONBOARDING_PREVIOUS,
    ONBOARDING_SKIP,
    ONBOARDING_FINISH,
    ONBOARDING_RECONCILED
  )
  pure val TOKENS = Set(0, 1, 2, 3, 99)

  var s: State

  pure def idle_phases: int -> int = Map(
    CALENDAR -> IDLE,
    WEATHER -> IDLE,
    STOCKS -> IDLE,
    NEWS -> IDLE,
    ONBOARDING_HYDRATION -> IDLE,
    DESKTOP_NOTIFICATION -> IDLE,
    CLOUD_NOTIFICATION -> IDLE,
    CLOUD_REMINDER_SYNC -> IDLE,
    BLUETOOTH -> IDLE
  )

  pure def zero_generations: int -> int = Map(
    CALENDAR -> 0,
    WEATHER -> 0,
    STOCKS -> 0,
    NEWS -> 0,
    ONBOARDING_HYDRATION -> 0,
    DESKTOP_NOTIFICATION -> 0,
    CLOUD_NOTIFICATION -> 0,
    CLOUD_REMINDER_SYNC -> 0,
    BLUETOOTH -> 0
  )

  pure def next_onboarding(step: int): int =
    if (step == WELCOME) ACCOUNT
    else if (step == ACCOUNT) BACKUP
    else if (step == BACKUP) ESSENTIALS
    else if (step == ESSENTIALS) ONBOARDING_READY
    else step

  pure def previous_onboarding(step: int): int =
    if (step == ACCOUNT) WELCOME
    else if (step == BACKUP) ACCOUNT
    else if (step == ESSENTIALS) BACKUP
    else if (step == ONBOARDING_READY) ESSENTIALS
    else step

  pure def clear_auth_lanes(current: State): State = {
    ...current,
    lane_phase: current.lane_phase
      .put(CALENDAR, IDLE)
      .put(ONBOARDING_HYDRATION, IDLE)
      .put(CLOUD_NOTIFICATION, IDLE)
      .put(CLOUD_REMINDER_SYNC, IDLE),
    lane_token: current.lane_token
      .put(CALENDAR, 0)
      .put(ONBOARDING_HYDRATION, 0)
      .put(CLOUD_NOTIFICATION, 0)
      .put(CLOUD_REMINDER_SYNC, 0),
  }

  pure def begin_login(current: State): State =
    if (
      current.app_phase != READY
      or current.auth_phase == AUTHENTICATING
      or current.auth_phase == SIGNED_IN
      or current.generation >= MAX_GENERATION
    )
      current
    else {
      ...current,
      auth_phase: AUTHENTICATING,
      auth_generation: current.generation + 1,
      auth_token: current.generation + 1,
      generation: current.generation + 1,
    }

  pure def complete_login(current: State, token: int, succeeded: bool): State =
    if (
      current.auth_phase != AUTHENTICATING
      or current.auth_token != token
    )
      current
    else {
      ...current,
      auth_phase: if (succeeded) SIGNED_IN else AUTH_FAILED,
      auth_token: 0,
    }

  pure def logout(current: State): State =
    if (current.app_phase != READY)
      current
    else {
      ...clear_auth_lanes(current),
      auth_phase: SIGNED_OUT,
      auth_token: 0,
    }

  pure def begin_lane(current: State, lane: int): State =
    if (
      current.app_phase != READY
      or not(LANES.contains(lane))
      or current.lane_phase.get(lane) == RUNNING
      or (AUTH_LANES.contains(lane) and current.auth_phase != SIGNED_IN)
      or current.generation >= MAX_GENERATION
    )
      current
    else {
      ...current,
      generation: current.generation + 1,
      lane_phase: current.lane_phase.put(lane, RUNNING),
      lane_generation: current.lane_generation.put(lane, current.generation + 1),
      lane_token: current.lane_token.put(lane, current.generation + 1),
    }

  pure def complete_lane(
    current: State,
    lane: int,
    token: int,
    succeeded: bool
  ): State =
    if (
      not(LANES.contains(lane))
      or current.lane_phase.get(lane) != RUNNING
      or current.lane_token.get(lane) != token
    )
      current
    else {
      ...current,
      lane_phase: current.lane_phase.put(
        lane,
        if (succeeded) LANE_READY else LANE_FAILED
      ),
      lane_token: current.lane_token.put(lane, 0),
    }

  // The complete transition relation. It is defined for every bounded
  // State x Event x Lane x Token x OnboardingStep input.
  pure def transition(
    current: State,
    event: int,
    lane: int,
    token: int,
    reconciled_step: int
  ): State =
    if (event == STARTUP_COMPLETED)
      if (current.app_phase == BOOTING)
        { ...current, app_phase: READY }
      else
        current
    else if (current.app_phase != READY)
      current
    else if (event == LOGIN_REQUESTED)
      begin_login(current)
    else if (event == LOGIN_SUCCEEDED)
      complete_login(current, token, true)
    else if (event == LOGIN_FAILED)
      complete_login(current, token, false)
    else if (event == LOGOUT_REQUESTED)
      logout(current)
    else if (event == LANE_REQUESTED)
      begin_lane(current, lane)
    else if (event == LANE_SUCCEEDED)
      complete_lane(current, lane, token, true)
    else if (event == LANE_FAILED_EVENT)
      complete_lane(current, lane, token, false)
    else if (event == ONBOARDING_NEXT)
      { ...current, onboarding: next_onboarding(current.onboarding) }
    else if (event == ONBOARDING_PREVIOUS)
      { ...current, onboarding: previous_onboarding(current.onboarding) }
    else if (event == ONBOARDING_SKIP)
      if (current.onboarding == COMPLETE)
        current
      else
        { ...current, onboarding: ONBOARDING_READY }
    else if (event == ONBOARDING_FINISH)
      if (current.onboarding == ONBOARDING_READY)
        {
          ...current,
          onboarding: COMPLETE,
          onboarding_completed_once: true,
        }
      else
        current
    else if (event == ONBOARDING_RECONCILED)
      if (
        not(ONBOARDING_STEPS.contains(reconciled_step))
        or (current.onboarding == COMPLETE and reconciled_step != COMPLETE)
      )
        current
      else
        {
          ...current,
          onboarding: reconciled_step,
          onboarding_completed_once:
            current.onboarding_completed_once or reconciled_step == COMPLETE,
        }
    else
      current

  action init_from_persisted(
    persisted_auth: int,
    persisted_onboarding: int
  ): bool = all {
    Set(SIGNED_OUT, SIGNED_IN).contains(persisted_auth),
    ONBOARDING_STEPS.contains(persisted_onboarding),
    s' = {
      app_phase: BOOTING,
      auth_phase: persisted_auth,
      auth_generation: 0,
      auth_token: 0,
      onboarding: persisted_onboarding,
      onboarding_completed_once: persisted_onboarding == COMPLETE,
      generation: 0,
      lane_phase: idle_phases,
      lane_generation: zero_generations,
      lane_token: zero_generations,
    },
  }

  action init_signed_out: bool =
    init_from_persisted(SIGNED_OUT, WELCOME)

  action init: bool = {
    nondet persisted_auth = Set(SIGNED_OUT, SIGNED_IN).oneOf()
    nondet persisted_onboarding = ONBOARDING_STEPS.oneOf()
    init_from_persisted(persisted_auth, persisted_onboarding)
  }

  action apply(
    event: int,
    lane: int,
    token: int,
    reconciled_step: int
  ): bool = all {
    EVENTS.contains(event),
    LANES.contains(lane),
    TOKENS.contains(token),
    ONBOARDING_STEPS.contains(reconciled_step),
    s' = transition(s, event, lane, token, reconciled_step),
  }

  action step: bool = {
    nondet event = EVENTS.oneOf()
    nondet lane = LANES.oneOf()
    nondet token = TOKENS.oneOf()
    nondet reconciled_step = ONBOARDING_STEPS.oneOf()
    apply(event, lane, token, reconciled_step)
  }

  val domains_are_bounded = and {
    APP_PHASES.contains(s.app_phase),
    AUTH_PHASES.contains(s.auth_phase),
    ONBOARDING_STEPS.contains(s.onboarding),
    s.generation >= 0,
    s.generation <= MAX_GENERATION,
    s.auth_generation >= 0,
    s.auth_generation <= s.generation,
    s.auth_token >= 0,
    s.auth_token <= s.generation,
    LANES.forall(lane => LANE_PHASES.contains(s.lane_phase.get(lane))),
    LANES.forall(lane => s.lane_generation.get(lane) >= 0),
    LANES.forall(lane => s.lane_generation.get(lane) <= s.generation),
    LANES.forall(lane => s.lane_token.get(lane) >= 0),
    LANES.forall(lane => s.lane_token.get(lane) <= s.generation),
  }

  val auth_token_is_coherent = and {
    (s.auth_phase == AUTHENTICATING) == (s.auth_token != 0),
    (s.auth_token != 0).implies(s.auth_token == s.auth_generation),
  }

  val lane_tokens_are_coherent = LANES.forall(lane => and {
    (s.lane_phase.get(lane) == RUNNING) == (s.lane_token.get(lane) != 0),
    (s.lane_token.get(lane) != 0).implies(
      s.lane_token.get(lane) == s.lane_generation.get(lane)
    ),
  })

  val auth_lanes_fail_closed = AUTH_LANES.forall(lane =>
    (s.lane_phase.get(lane) == RUNNING).implies(s.auth_phase == SIGNED_IN)
  )

  val booting_has_no_operations =
    (s.app_phase == BOOTING).implies(
      LANES.forall(lane => s.lane_phase.get(lane) != RUNNING)
    )

  // This proof witness is irreversible: transition branches may set it to
  // true but never clear it. A future accidental transition away from COMPLETE
  // would therefore violate this invariant immediately.
  val onboarding_completion_is_monotonic =
    s.onboarding_completed_once == (s.onboarding == COMPLETE)

  val app_state_safety = and {
    domains_are_bounded,
    auth_token_is_coherent,
    lane_tokens_are_coherent,
    auth_lanes_fail_closed,
    booting_has_no_operations,
    onboarding_completion_is_monotonic,
  }

  val booting_reached = s.app_phase == BOOTING
  val ready_reached = s.app_phase == READY
  val signed_out_reached = s.auth_phase == SIGNED_OUT
  val authenticating_reached = s.auth_phase == AUTHENTICATING
  val signed_in_reached = s.auth_phase == SIGNED_IN
  val auth_failed_reached = s.auth_phase == AUTH_FAILED
  val lane_idle_reached = LANES.exists(lane => s.lane_phase.get(lane) == IDLE)
  val lane_running_reached = LANES.exists(lane => s.lane_phase.get(lane) == RUNNING)
  val lane_ready_reached = LANES.exists(lane => s.lane_phase.get(lane) == LANE_READY)
  val lane_failed_reached = LANES.exists(lane => s.lane_phase.get(lane) == LANE_FAILED)
  val onboarding_complete_reached = s.onboarding == COMPLETE
}
