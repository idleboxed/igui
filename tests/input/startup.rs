// SPDX-License-Identifier: BSD-3-Clause
use igui::input::startup::{StartupInputTest, Transition};

#[test]
fn startup_timer_begins_after_ready_and_is_consumed_once() {
    let mut startup = StartupInputTest::default();
    assert_eq!(startup.update(50_000, false, false, false), Transition::None);
    startup.update(60_000, true, false, false);
    startup.update(69_999, true, false, false);
    assert_eq!(startup.countdown(), None);
    startup.update(70_000, true, false, false);
    assert_eq!(startup.countdown(), Some(10));
    assert_eq!(startup.message().unwrap(), "Controller test starts in 10 ...\nPress B to cancel.");
    startup.update(79_999, true, false, false);
    assert_eq!(startup.countdown(), Some(1));
    assert_eq!(startup.update(80_000, true, false, false), Transition::Start);
    assert_eq!(startup.update(120_000, true, false, false), Transition::None);
    assert_eq!(startup.countdown(), None);
}

#[test]
fn activity_before_invitation_and_b_at_deadline_cancel_without_reopening() {
    let mut startup = StartupInputTest::default();
    startup.update(0, true, false, false);
    startup.update(9_999, true, true, false);
    assert_eq!(startup.update(20_000, true, false, false), Transition::None);
    assert_eq!(startup.countdown(), None);
    let mut startup = StartupInputTest::default();
    startup.update(0, true, false, false);
    startup.update(10_000, true, false, false);
    startup.update(15_000, true, true, false);
    assert_eq!(startup.countdown(), Some(5));
    assert_eq!(startup.update(20_000, true, true, true), Transition::Cancelled);
    assert_eq!(startup.update(100_000, true, false, false), Transition::None);
}

#[test]
fn delayed_first_countdown_frame_still_gives_ten_seconds_to_cancel() {
    for first_frame in [14_000, 30_000] {
        let mut startup = StartupInputTest::default();
        startup.update(0, true, false, false);
        assert_eq!(startup.update(first_frame, true, false, false), Transition::None);
        assert_eq!(startup.countdown(), Some(10));
        startup.update(first_frame + 9_999, true, false, false);
        assert_eq!(startup.countdown(), Some(1));
        assert_eq!(startup.update(first_frame + 10_000, true, false, false), Transition::Start);
    }
}
