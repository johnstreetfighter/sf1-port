use sf_engine::front::{FrameInputs, GameSession, Transition};

/// Accept the selected fighter's default style, using the same confirm input
/// as the portrait grid. Keep the caller's handling of the final press.
pub fn confirm_fighter<T>(
    session: &mut GameSession,
    input: FrameInputs,
    finish: impl FnOnce(&mut GameSession, FrameInputs) -> T,
) -> T {
    if session.match_config().era_select_enabled() {
        let open = session.step(input);
        assert_eq!(open.transition, Some(Transition::StyleOpened));
        session.step(FrameInputs::default());
    }
    finish(session, input)
}
