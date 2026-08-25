module app_state_test {
  import app_state.* from "./app_state"

  run startupLoginAndCalendarTest =
    init_signed_out
      .then(apply(STARTUP_COMPLETED, CALENDAR, 0, WELCOME))
      .then(apply(LOGIN_REQUESTED, CALENDAR, 0, WELCOME))
      .then(apply(LOGIN_SUCCEEDED, CALENDAR, 1, WELCOME))
      .then(apply(LANE_REQUESTED, CALENDAR, 0, WELCOME))
      .then(apply(LANE_SUCCEEDED, CALENDAR, 2, WELCOME))
      .expect(and {
        s.app_phase == READY,
        s.auth_phase == SIGNED_IN,
        s.lane_phase.get(CALENDAR) == LANE_READY,
        app_state_safety,
      })

  run staleLoginCannotResurrectAfterLogoutTest =
    init_signed_out
      .then(apply(STARTUP_COMPLETED, CALENDAR, 0, WELCOME))
      .then(apply(LOGIN_REQUESTED, CALENDAR, 0, WELCOME))
      .then(apply(LOGOUT_REQUESTED, CALENDAR, 0, WELCOME))
      .then(apply(LOGIN_SUCCEEDED, CALENDAR, 1, WELCOME))
      .expect(and {
        s.auth_phase == SIGNED_OUT,
        s.auth_token == 0,
        app_state_safety,
      })

  run logoutCancelsAuthenticatedLaneTest =
    init_signed_out
      .then(apply(STARTUP_COMPLETED, CALENDAR, 0, WELCOME))
      .then(apply(LOGIN_REQUESTED, CALENDAR, 0, WELCOME))
      .then(apply(LOGIN_SUCCEEDED, CALENDAR, 1, WELCOME))
      .then(apply(LANE_REQUESTED, CALENDAR, 0, WELCOME))
      .then(apply(LOGOUT_REQUESTED, CALENDAR, 0, WELCOME))
      .then(apply(LANE_SUCCEEDED, CALENDAR, 2, WELCOME))
      .expect(and {
        s.auth_phase == SIGNED_OUT,
        s.lane_phase.get(CALENDAR) == IDLE,
        s.lane_token.get(CALENDAR) == 0,
        app_state_safety,
      })

  run publicLanesRemainIndependentTest =
    init_signed_out
      .then(apply(STARTUP_COMPLETED, WEATHER, 0, WELCOME))
      .then(apply(LANE_REQUESTED, WEATHER, 0, WELCOME))
      .then(apply(LANE_REQUESTED, NEWS, 0, WELCOME))
      .then(apply(LANE_FAILED_EVENT, WEATHER, 1, WELCOME))
      .expect(and {
        s.lane_phase.get(WEATHER) == LANE_FAILED,
        s.lane_phase.get(NEWS) == RUNNING,
        s.lane_token.get(NEWS) == 2,
        app_state_safety,
      })

  run bluetoothEffectUsesItsOwnTokenLaneTest =
    init_signed_out
      .then(apply(STARTUP_COMPLETED, BLUETOOTH, 0, WELCOME))
      .then(apply(LANE_REQUESTED, BLUETOOTH, 0, WELCOME))
      .then(apply(LANE_SUCCEEDED, BLUETOOTH, 1, WELCOME))
      .expect(and {
        s.lane_phase.get(BLUETOOTH) == LANE_READY,
        s.lane_token.get(BLUETOOTH) == 0,
        app_state_safety,
      })

  run onboardingFollowsDeclaredEdgesTest =
    init_signed_out
      .then(apply(STARTUP_COMPLETED, WEATHER, 0, WELCOME))
      .then(apply(ONBOARDING_FINISH, WEATHER, 0, WELCOME))
      .then(apply(ONBOARDING_NEXT, WEATHER, 0, WELCOME))
      .then(apply(ONBOARDING_NEXT, WEATHER, 0, WELCOME))
      .then(apply(ONBOARDING_NEXT, WEATHER, 0, WELCOME))
      .then(apply(ONBOARDING_NEXT, WEATHER, 0, WELCOME))
      .then(apply(ONBOARDING_FINISH, WEATHER, 0, WELCOME))
      .expect(and {
        s.onboarding == COMPLETE,
        app_state_safety,
      })

  run completedOnboardingCannotRegressTest =
    init_signed_out
      .then(apply(STARTUP_COMPLETED, WEATHER, 0, WELCOME))
      .then(apply(ONBOARDING_SKIP, WEATHER, 0, WELCOME))
      .then(apply(ONBOARDING_FINISH, WEATHER, 0, WELCOME))
      .then(apply(ONBOARDING_RECONCILED, WEATHER, 0, ACCOUNT))
      .expect(and {
        s.onboarding == COMPLETE,
        app_state_safety,
      })

  run preStartupRequestsFailClosedTest =
    init_signed_out
      .then(apply(LOGIN_REQUESTED, CALENDAR, 0, WELCOME))
      .then(apply(LANE_REQUESTED, WEATHER, 0, WELCOME))
      .then(apply(ONBOARDING_NEXT, WEATHER, 0, ACCOUNT))
      .expect(and {
        s.app_phase == BOOTING,
        s.auth_phase == SIGNED_OUT,
        s.onboarding == WELCOME,
        s.generation == 0,
        s.lane_phase.get(WEATHER) == IDLE,
        app_state_safety,
      })

  run staleSameLaneCompletionCannotOverwriteNewRequestTest =
    init_signed_out
      .then(apply(STARTUP_COMPLETED, WEATHER, 0, WELCOME))
      .then(apply(LANE_REQUESTED, WEATHER, 0, WELCOME))
      .then(apply(LANE_SUCCEEDED, WEATHER, 1, WELCOME))
      .then(apply(LANE_REQUESTED, WEATHER, 0, WELCOME))
      .then(apply(LANE_FAILED_EVENT, WEATHER, 1, WELCOME))
      .expect(and {
        s.lane_phase.get(WEATHER) == RUNNING,
        s.lane_token.get(WEATHER) == 2,
        app_state_safety,
      })

  run boundedGenerationExhaustionStuttersTest =
    init_signed_out
      .then(apply(STARTUP_COMPLETED, WEATHER, 0, WELCOME))
      .then(apply(LANE_REQUESTED, WEATHER, 0, WELCOME))
      .then(apply(LANE_REQUESTED, STOCKS, 0, WELCOME))
      .then(apply(LANE_REQUESTED, NEWS, 0, WELCOME))
      .then(apply(LANE_REQUESTED, DESKTOP_NOTIFICATION, 0, WELCOME))
      .expect(and {
        s.generation == MAX_GENERATION,
        s.lane_phase.get(WEATHER) == RUNNING,
        s.lane_phase.get(STOCKS) == RUNNING,
        s.lane_phase.get(NEWS) == RUNNING,
        s.lane_phase.get(DESKTOP_NOTIFICATION) == IDLE,
        app_state_safety,
      })

  run persistedSessionAndOnboardingStartSafelyTest =
    init_from_persisted(SIGNED_IN, BACKUP)
      .then(apply(STARTUP_COMPLETED, WEATHER, 0, WELCOME))
      .expect(and {
        s.app_phase == READY,
        s.auth_phase == SIGNED_IN,
        s.auth_token == 0,
        s.onboarding == BACKUP,
        s.generation == 0,
        app_state_safety,
      })
}
