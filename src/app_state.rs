//! Total application control-state machine shared by the desktop runtime and
//! the language-neutral formal model in `formal/app_state.qnt`.
//!
//! All fields are private. The only way to change control state is `dispatch`,
//! which computes a candidate state, validates every invariant, and commits the
//! candidate atomically. Unsupported events fail closed; late async results are
//! classified as stale and stutter.

use serde::Serialize;

pub const LANE_COUNT: usize = 9;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AppPhase {
    Booting,
    Ready,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthPhase {
    SignedOut,
    Authenticating,
    SignedIn,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Provider {
    Google,
    Apple,
    Azure,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OnboardingStep {
    Welcome,
    Account,
    Backup,
    Essentials,
    Ready,
    Complete,
}

impl OnboardingStep {
    #[cfg(test)]
    pub const ALL: [Self; 6] = [
        Self::Welcome,
        Self::Account,
        Self::Backup,
        Self::Essentials,
        Self::Ready,
        Self::Complete,
    ];

    pub fn from_persisted(step: &str, completed: bool) -> Self {
        if completed {
            return Self::Complete;
        }
        match step.trim() {
            "account" => Self::Account,
            "backup" => Self::Backup,
            "essentials" => Self::Essentials,
            "ready" => Self::Ready,
            _ => Self::Welcome,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Welcome => "welcome",
            Self::Account => "account",
            Self::Backup => "backup",
            Self::Essentials => "essentials",
            Self::Ready => "ready",
            Self::Complete => "complete",
        }
    }

    pub const fn index(self) -> u8 {
        match self {
            Self::Welcome => 0,
            Self::Account => 1,
            Self::Backup => 2,
            Self::Essentials => 3,
            Self::Ready | Self::Complete => 4,
        }
    }

    const fn next(self) -> Option<Self> {
        match self {
            Self::Welcome => Some(Self::Account),
            Self::Account => Some(Self::Backup),
            Self::Backup => Some(Self::Essentials),
            Self::Essentials => Some(Self::Ready),
            Self::Ready | Self::Complete => None,
        }
    }

    const fn previous(self) -> Option<Self> {
        match self {
            Self::Account => Some(Self::Welcome),
            Self::Backup => Some(Self::Account),
            Self::Essentials => Some(Self::Backup),
            Self::Ready => Some(Self::Essentials),
            Self::Welcome | Self::Complete => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[repr(usize)]
#[serde(rename_all = "snake_case")]
pub enum Lane {
    Calendar = 0,
    Weather = 1,
    Stocks = 2,
    News = 3,
    OnboardingHydration = 4,
    DesktopNotification = 5,
    CloudNotification = 6,
    CloudReminderSync = 7,
    Bluetooth = 8,
}

impl Lane {
    pub const ALL: [Self; LANE_COUNT] = [
        Self::Calendar,
        Self::Weather,
        Self::Stocks,
        Self::News,
        Self::OnboardingHydration,
        Self::DesktopNotification,
        Self::CloudNotification,
        Self::CloudReminderSync,
        Self::Bluetooth,
    ];

    pub const fn requires_auth(self) -> bool {
        matches!(
            self,
            Self::Calendar
                | Self::OnboardingHydration
                | Self::CloudNotification
                | Self::CloudReminderSync
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LanePhase {
    Idle,
    Running,
    Ready,
    Failed,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct LaneState {
    phase: LanePhase,
    generation: u64,
    active_token: u64,
}

impl Default for LaneState {
    fn default() -> Self {
        Self {
            phase: LanePhase::Idle,
            generation: 0,
            active_token: 0,
        }
    }
}

impl LaneState {
    #[cfg(test)]
    pub const fn phase(self) -> LanePhase {
        self.phase
    }

    pub const fn is_running(self) -> bool {
        matches!(self.phase, LanePhase::Running)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct AuthState {
    phase: AuthPhase,
    provider: Option<Provider>,
    generation: u64,
    active_token: u64,
}

impl AuthState {
    fn initial(signed_in: bool) -> Self {
        Self {
            phase: if signed_in {
                AuthPhase::SignedIn
            } else {
                AuthPhase::SignedOut
            },
            provider: None,
            generation: 0,
            active_token: 0,
        }
    }

    pub const fn phase(self) -> AuthPhase {
        self.phase
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct AppMachine {
    app_phase: AppPhase,
    auth: AuthState,
    onboarding: OnboardingStep,
    onboarding_completed_once: bool,
    generation: u64,
    lanes: [LaneState; LANE_COUNT],
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct OperationToken(u64);

impl OperationToken {
    pub const fn get(self) -> u64 {
        self.0
    }

    #[cfg(test)]
    pub const fn from_raw(value: u64) -> Self {
        Self(value)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum Event {
    StartupCompleted,
    LoginRequested(Provider),
    LoginSucceeded(OperationToken),
    LoginFailed(OperationToken),
    LogoutRequested,
    LaneRequested(Lane),
    LaneSucceeded(Lane, OperationToken),
    LaneFailed(Lane, OperationToken),
    OnboardingNext,
    OnboardingPrevious,
    OnboardingSkipToReady,
    OnboardingFinish,
    OnboardingReconciled(OnboardingStep),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RejectReason {
    AppNotReady,
    AlreadyReady,
    AuthRequired,
    AuthenticationInProgress,
    AlreadySignedIn,
    OperationInProgress,
    InvalidOnboardingTransition,
    GenerationExhausted,
    InvariantViolation,
}

impl RejectReason {
    pub const fn message(self) -> &'static str {
        match self {
            Self::AppNotReady => "The application is not ready",
            Self::AlreadyReady => "The application has already started",
            Self::AuthRequired => "Sign in before starting this operation",
            Self::AuthenticationInProgress => "A sign-in operation is already in progress",
            Self::AlreadySignedIn => "Sign out before starting another sign-in",
            Self::OperationInProgress => "This operation is already in progress",
            Self::InvalidOnboardingTransition => "That onboarding transition is not allowed",
            Self::GenerationExhausted => "The operation generation counter is exhausted",
            Self::InvariantViolation => {
                "The requested transition would violate application invariants"
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionOutcome {
    Applied(Option<OperationToken>),
    Stale,
    Rejected(RejectReason),
}

impl TransitionOutcome {
    pub const fn token(self) -> Option<OperationToken> {
        match self {
            Self::Applied(token) => token,
            Self::Stale | Self::Rejected(_) => None,
        }
    }

    pub const fn committed(self) -> bool {
        matches!(self, Self::Applied(_))
    }
}

impl AppMachine {
    pub fn new(signed_in: bool, onboarding: OnboardingStep) -> Self {
        Self {
            app_phase: AppPhase::Booting,
            auth: AuthState::initial(signed_in),
            onboarding,
            onboarding_completed_once: onboarding == OnboardingStep::Complete,
            generation: 0,
            lanes: [LaneState::default(); LANE_COUNT],
        }
    }

    pub const fn auth_phase(&self) -> AuthPhase {
        self.auth.phase()
    }

    pub const fn is_signed_in(&self) -> bool {
        matches!(self.auth.phase, AuthPhase::SignedIn)
    }

    pub const fn authentication_in_progress(&self) -> bool {
        matches!(self.auth.phase, AuthPhase::Authenticating)
    }

    pub const fn onboarding(&self) -> OnboardingStep {
        self.onboarding
    }

    pub fn lane(&self, lane: Lane) -> LaneState {
        self.lanes[lane as usize]
    }

    pub fn accepts_auth_token(&self, token: OperationToken) -> bool {
        self.auth.phase == AuthPhase::Authenticating && self.auth.active_token == token.get()
    }

    pub fn accepts_lane_token(&self, lane: Lane, token: OperationToken) -> bool {
        let state = self.lane(lane);
        state.phase == LanePhase::Running && state.active_token == token.get()
    }

    pub fn dispatch(&mut self, event: Event) -> TransitionOutcome {
        if self.check_invariants().is_err() {
            return TransitionOutcome::Rejected(RejectReason::InvariantViolation);
        }

        let mut candidate = self.clone();
        let outcome = candidate.apply(event);
        if !outcome.committed() {
            return outcome;
        }
        if candidate.check_invariants().is_err() {
            return TransitionOutcome::Rejected(RejectReason::InvariantViolation);
        }
        *self = candidate;
        outcome
    }

    pub fn check_invariants(&self) -> Result<(), &'static str> {
        let auth_is_active = self.auth.active_token != 0;
        if auth_is_active != matches!(self.auth.phase, AuthPhase::Authenticating) {
            return Err("authentication phase/token mismatch");
        }
        if auth_is_active
            && (self.auth.active_token != self.auth.generation
                || self.auth.active_token > self.generation)
        {
            return Err("authentication token is not the current generation");
        }
        if self.auth.provider.is_some() != auth_is_active {
            return Err("authentication provider exists outside an active login");
        }
        if self.onboarding_completed_once != (self.onboarding == OnboardingStep::Complete) {
            return Err("completed onboarding regressed or lost its completion witness");
        }

        for lane in Lane::ALL {
            let state = self.lane(lane);
            let active = state.active_token != 0;
            if active != matches!(state.phase, LanePhase::Running) {
                return Err("lane phase/token mismatch");
            }
            if active
                && (state.active_token != state.generation || state.active_token > self.generation)
            {
                return Err("lane token is not the current generation");
            }
            if lane.requires_auth() && active && !self.is_signed_in() {
                return Err("auth-bound lane is active while signed out");
            }
        }

        if self.app_phase == AppPhase::Booting && self.lanes.iter().any(|state| state.is_running())
        {
            return Err("operation started before application readiness");
        }
        Ok(())
    }

    fn apply(&mut self, event: Event) -> TransitionOutcome {
        if !matches!(event, Event::StartupCompleted) && self.app_phase != AppPhase::Ready {
            return TransitionOutcome::Rejected(RejectReason::AppNotReady);
        }

        match event {
            Event::StartupCompleted => {
                if self.app_phase == AppPhase::Ready {
                    TransitionOutcome::Rejected(RejectReason::AlreadyReady)
                } else {
                    self.app_phase = AppPhase::Ready;
                    TransitionOutcome::Applied(None)
                }
            }
            Event::LoginRequested(provider) => self.begin_login(provider),
            Event::LoginSucceeded(token) => self.complete_login(token, true),
            Event::LoginFailed(token) => self.complete_login(token, false),
            Event::LogoutRequested => {
                self.auth = AuthState {
                    phase: AuthPhase::SignedOut,
                    provider: None,
                    generation: self.auth.generation,
                    active_token: 0,
                };
                for lane in Lane::ALL.into_iter().filter(|lane| lane.requires_auth()) {
                    let state = &mut self.lanes[lane as usize];
                    state.phase = LanePhase::Idle;
                    state.active_token = 0;
                }
                TransitionOutcome::Applied(None)
            }
            Event::LaneRequested(lane) => self.begin_lane(lane),
            Event::LaneSucceeded(lane, token) => self.complete_lane(lane, token, true),
            Event::LaneFailed(lane, token) => self.complete_lane(lane, token, false),
            Event::OnboardingNext => match self.onboarding.next() {
                Some(next) => {
                    self.onboarding = next;
                    TransitionOutcome::Applied(None)
                }
                None => TransitionOutcome::Rejected(RejectReason::InvalidOnboardingTransition),
            },
            Event::OnboardingPrevious => match self.onboarding.previous() {
                Some(previous) => {
                    self.onboarding = previous;
                    TransitionOutcome::Applied(None)
                }
                None => TransitionOutcome::Rejected(RejectReason::InvalidOnboardingTransition),
            },
            Event::OnboardingSkipToReady => {
                if self.onboarding == OnboardingStep::Complete {
                    TransitionOutcome::Rejected(RejectReason::InvalidOnboardingTransition)
                } else {
                    self.onboarding = OnboardingStep::Ready;
                    TransitionOutcome::Applied(None)
                }
            }
            Event::OnboardingFinish => {
                if self.onboarding == OnboardingStep::Ready {
                    self.onboarding = OnboardingStep::Complete;
                    self.onboarding_completed_once = true;
                    TransitionOutcome::Applied(None)
                } else {
                    TransitionOutcome::Rejected(RejectReason::InvalidOnboardingTransition)
                }
            }
            Event::OnboardingReconciled(step) => {
                if self.onboarding == OnboardingStep::Complete && step != OnboardingStep::Complete {
                    TransitionOutcome::Stale
                } else {
                    self.onboarding = step;
                    if step == OnboardingStep::Complete {
                        self.onboarding_completed_once = true;
                    }
                    TransitionOutcome::Applied(None)
                }
            }
        }
    }

    fn begin_login(&mut self, provider: Provider) -> TransitionOutcome {
        match self.auth.phase {
            AuthPhase::Authenticating => {
                return TransitionOutcome::Rejected(RejectReason::AuthenticationInProgress)
            }
            AuthPhase::SignedIn => {
                return TransitionOutcome::Rejected(RejectReason::AlreadySignedIn)
            }
            AuthPhase::SignedOut | AuthPhase::Failed => {}
        }

        let Some(token) = self.next_token() else {
            return TransitionOutcome::Rejected(RejectReason::GenerationExhausted);
        };
        self.auth = AuthState {
            phase: AuthPhase::Authenticating,
            provider: Some(provider),
            generation: token.get(),
            active_token: token.get(),
        };
        TransitionOutcome::Applied(Some(token))
    }

    fn complete_login(&mut self, token: OperationToken, succeeded: bool) -> TransitionOutcome {
        if !self.accepts_auth_token(token) {
            return TransitionOutcome::Stale;
        }
        self.auth.phase = if succeeded {
            AuthPhase::SignedIn
        } else {
            AuthPhase::Failed
        };
        self.auth.provider = None;
        self.auth.active_token = 0;
        TransitionOutcome::Applied(None)
    }

    fn begin_lane(&mut self, lane: Lane) -> TransitionOutcome {
        if lane.requires_auth() && !self.is_signed_in() {
            return TransitionOutcome::Rejected(RejectReason::AuthRequired);
        }
        if self.lane(lane).is_running() {
            return TransitionOutcome::Rejected(RejectReason::OperationInProgress);
        }
        let Some(token) = self.next_token() else {
            return TransitionOutcome::Rejected(RejectReason::GenerationExhausted);
        };
        self.lanes[lane as usize] = LaneState {
            phase: LanePhase::Running,
            generation: token.get(),
            active_token: token.get(),
        };
        TransitionOutcome::Applied(Some(token))
    }

    fn complete_lane(
        &mut self,
        lane: Lane,
        token: OperationToken,
        succeeded: bool,
    ) -> TransitionOutcome {
        if !self.accepts_lane_token(lane, token) {
            return TransitionOutcome::Stale;
        }
        let state = &mut self.lanes[lane as usize];
        state.phase = if succeeded {
            LanePhase::Ready
        } else {
            LanePhase::Failed
        };
        state.active_token = 0;
        TransitionOutcome::Applied(None)
    }

    fn next_token(&mut self) -> Option<OperationToken> {
        self.generation = self.generation.checked_add(1)?;
        Some(OperationToken(self.generation))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::{HashSet, VecDeque};

    fn ready_machine(signed_in: bool) -> AppMachine {
        let mut machine = AppMachine::new(signed_in, OnboardingStep::Welcome);
        assert_eq!(
            machine.dispatch(Event::StartupCompleted),
            TransitionOutcome::Applied(None)
        );
        machine
    }

    #[test]
    fn stale_login_cannot_resurrect_session_after_logout() {
        let mut machine = ready_machine(false);
        let token = machine
            .dispatch(Event::LoginRequested(Provider::Google))
            .token()
            .unwrap();
        machine.dispatch(Event::LogoutRequested);
        assert_eq!(
            machine.dispatch(Event::LoginSucceeded(token)),
            TransitionOutcome::Stale
        );
        assert_eq!(machine.auth_phase(), AuthPhase::SignedOut);
        assert!(machine.check_invariants().is_ok());
    }

    #[test]
    fn effects_and_authentication_fail_closed_before_startup() {
        let mut machine = AppMachine::new(false, OnboardingStep::Welcome);
        let initial = machine.clone();
        assert_eq!(
            machine.dispatch(Event::LoginRequested(Provider::Google)),
            TransitionOutcome::Rejected(RejectReason::AppNotReady)
        );
        assert_eq!(
            machine.dispatch(Event::LaneRequested(Lane::Weather)),
            TransitionOutcome::Rejected(RejectReason::AppNotReady)
        );
        assert_eq!(
            machine.dispatch(Event::OnboardingNext),
            TransitionOutcome::Rejected(RejectReason::AppNotReady)
        );
        assert_eq!(machine, initial);
    }

    #[test]
    fn exhausted_generation_rejects_without_partial_mutation() {
        let mut machine = ready_machine(false);
        machine.generation = u64::MAX;
        let initial = machine.clone();
        assert_eq!(
            machine.dispatch(Event::LaneRequested(Lane::Weather)),
            TransitionOutcome::Rejected(RejectReason::GenerationExhausted)
        );
        assert_eq!(machine, initial);
        assert!(machine.check_invariants().is_ok());
    }

    #[test]
    fn stale_completion_cannot_overwrite_a_newer_same_lane_request() {
        let mut machine = ready_machine(false);
        let first = machine
            .dispatch(Event::LaneRequested(Lane::Weather))
            .token()
            .unwrap();
        assert!(machine
            .dispatch(Event::LaneSucceeded(Lane::Weather, first))
            .committed());
        let second = machine
            .dispatch(Event::LaneRequested(Lane::Weather))
            .token()
            .unwrap();

        assert_eq!(
            machine.dispatch(Event::LaneFailed(Lane::Weather, first)),
            TransitionOutcome::Stale
        );
        assert!(machine.accepts_lane_token(Lane::Weather, second));
        assert_eq!(machine.lane(Lane::Weather).phase(), LanePhase::Running);
    }

    #[test]
    fn logout_cancels_auth_bound_lanes_and_stale_results_stutter() {
        let mut machine = ready_machine(true);
        let calendar = machine
            .dispatch(Event::LaneRequested(Lane::Calendar))
            .token()
            .unwrap();
        let weather = machine
            .dispatch(Event::LaneRequested(Lane::Weather))
            .token()
            .unwrap();
        machine.dispatch(Event::LogoutRequested);
        assert_eq!(machine.lane(Lane::Calendar).phase(), LanePhase::Idle);
        assert_eq!(machine.lane(Lane::Weather).phase(), LanePhase::Running);
        assert_eq!(
            machine.dispatch(Event::LaneSucceeded(Lane::Calendar, calendar)),
            TransitionOutcome::Stale
        );
        assert!(machine
            .dispatch(Event::LaneSucceeded(Lane::Weather, weather))
            .committed());
        assert!(machine.check_invariants().is_ok());
    }

    #[test]
    fn onboarding_is_strict_and_completion_is_monotonic() {
        let mut machine = ready_machine(false);
        assert_eq!(
            machine.dispatch(Event::OnboardingFinish),
            TransitionOutcome::Rejected(RejectReason::InvalidOnboardingTransition)
        );
        for _ in 0..4 {
            assert!(machine.dispatch(Event::OnboardingNext).committed());
        }
        assert_eq!(machine.onboarding(), OnboardingStep::Ready);
        assert!(machine.dispatch(Event::OnboardingFinish).committed());
        assert_eq!(machine.onboarding(), OnboardingStep::Complete);
        assert_eq!(
            machine.dispatch(Event::OnboardingReconciled(OnboardingStep::Welcome)),
            TransitionOutcome::Stale
        );
        assert_eq!(machine.onboarding(), OnboardingStep::Complete);
    }

    #[test]
    fn independent_lanes_do_not_overwrite_each_other() {
        let mut machine = ready_machine(true);
        let weather = machine
            .dispatch(Event::LaneRequested(Lane::Weather))
            .token()
            .unwrap();
        let news = machine
            .dispatch(Event::LaneRequested(Lane::News))
            .token()
            .unwrap();
        assert!(machine
            .dispatch(Event::LaneFailed(Lane::Weather, weather))
            .committed());
        assert_eq!(machine.lane(Lane::Weather).phase(), LanePhase::Failed);
        assert_eq!(machine.lane(Lane::News).phase(), LanePhase::Running);
        assert!(machine
            .dispatch(Event::LaneSucceeded(Lane::News, news))
            .committed());
        assert_eq!(machine.lane(Lane::News).phase(), LanePhase::Ready);
    }

    fn bounded_events(max_token: u64) -> Vec<Event> {
        let mut events = vec![
            Event::StartupCompleted,
            Event::LoginRequested(Provider::Google),
            Event::LoginRequested(Provider::Apple),
            Event::LoginRequested(Provider::Azure),
            Event::LogoutRequested,
            Event::OnboardingNext,
            Event::OnboardingPrevious,
            Event::OnboardingSkipToReady,
            Event::OnboardingFinish,
        ];
        events.extend(
            OnboardingStep::ALL
                .into_iter()
                .map(Event::OnboardingReconciled),
        );
        for raw in 0..=max_token {
            let token = OperationToken::from_raw(raw);
            events.push(Event::LoginSucceeded(token));
            events.push(Event::LoginFailed(token));
            for lane in Lane::ALL {
                events.push(Event::LaneSucceeded(lane, token));
                events.push(Event::LaneFailed(lane, token));
            }
        }
        events.extend(Lane::ALL.into_iter().map(Event::LaneRequested));
        events
    }

    #[test]
    fn bounded_reachable_graph_is_total_deterministic_and_invariant_safe() {
        const MAX_GENERATION: u64 = 2;
        const SAFETY_CAP: usize = 50_000;

        let events = bounded_events(MAX_GENERATION);
        let mut queue = VecDeque::new();
        for signed_in in [false, true] {
            for onboarding in OnboardingStep::ALL {
                queue.push_back(AppMachine::new(signed_in, onboarding));
            }
        }
        let mut visited = HashSet::new();
        let mut auth_phases = HashSet::new();
        let mut lane_phases = HashSet::new();
        let mut onboarding_steps = HashSet::new();

        while let Some(state) = queue.pop_front() {
            if !visited.insert(state.clone()) {
                continue;
            }
            assert!(visited.len() < SAFETY_CAP, "state-space safety cap reached");
            state.check_invariants().unwrap();
            auth_phases.insert(state.auth_phase());
            onboarding_steps.insert(state.onboarding());
            for lane in Lane::ALL {
                lane_phases.insert(state.lane(lane).phase());
            }

            for event in &events {
                let mut left = state.clone();
                let mut right = state.clone();
                let left_outcome = left.dispatch(*event);
                let right_outcome = right.dispatch(*event);
                assert_eq!(left_outcome, right_outcome);
                assert_eq!(left, right);
                left.check_invariants().unwrap();
                if left.generation <= MAX_GENERATION && !visited.contains(&left) {
                    queue.push_back(left);
                }
            }
        }

        assert!(
            visited.len() > 1_000,
            "exploration was unexpectedly shallow"
        );
        assert_eq!(auth_phases.len(), 4);
        assert_eq!(lane_phases.len(), 4);
        assert_eq!(onboarding_steps.len(), OnboardingStep::ALL.len());
    }
}
