# Windows final-window shutdown and frameless fullscreen

17.10.2026 fullscreen correction: WM_NCCALCSIZE already reads the fullscreen
state, but the maximized frameless branch applied rcWork unconditionally. Gate
that branch with !is_fullscreen. Fullscreen retains the monitor client rectangle
without a visible unmaximize/restore workaround in the application. Normal
frameless maximization still preserves the taskbar/work area. The existing
Tao fullscreen/WINDOWPLACEMENT mechanism owns entry and restoration; no new
native window controller or changes to process shutdown are introduced.

Acceptance: normal/maximized/already-fullscreen viewer; monitor/client geometry,
transition capture without intermediate smaller window, restored placement.

## Original shutdown correction

Base: unchanged crates.io Tao 0.35.3 sources, Apache-2.0 (included LICENSE).
Only functional change: `src/platform_impl/windows/event_loop.rs`, `run_return`.

After DispatchMessage returns and the runner requests ExitWithCode, leave the
outer message loop without waiting for RunnerState::Idle. Closing the last
visible window can leave the runner in HandlingMainEvents, with no remaining
message to produce the idle transition. Previously GetMessageW then blocked
indefinitely even though Tauri had emitted ExitRequested and removed all windows.

The existing EventLoopRunner::loop_destroyed()/move_state_to(Destroyed) flushes
MainEventsCleared and RedrawEventsCleared before LoopDestroyed from either active
state. DispatchMessage has returned before this change acts, so no native callback
is interrupted. Normal exit cleanup and exit codes remain owned by Tao/Tauri.
No timeout, forced termination, hidden window or application-level exit forwarding
is introduced. Vendoring keeps the fix reproducible without editing the user's
global Cargo cache. The locked compatible dependency version remains 0.35.3.

Reference: https://github.com/tauri-apps/tao/blob/tao-v0.35.3/src/platform_impl/windows/event_loop.rs
and the sibling event_loop/runner.rs state transitions.

Acceptance: real OS WM_CLOSE after disconnecting CDP must exit normally; multiple
editor windows must close and release the single-instance lock; a fresh process
must reopen the persisted project and its globally shared history-strip height.
Diagnostic trace instrumentation in the application is temporary and is removed
after these checks.
