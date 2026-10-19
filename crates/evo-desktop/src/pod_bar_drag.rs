//! Small, persistent drag state for the pod-bar teaching gesture.
//!
//! The app owns the visual shelf. This module only tracks the primary pointer
//! across egui frames and reports the release point so the app can teach the
//! active pod a frame.

use egui::{Context, Id, PointerButton, Pos2};

#[derive(Clone, Default)]
struct State {
    pressed: bool,
    last_pos: Option<Pos2>,
}

/// Returns the pointer release position for one completed primary-button
/// gesture. State is stored with egui's temporary data API, which survives
/// frames without leaking into application state.
pub fn update_state(ctx: &Context, id: Id) -> Option<Pos2> {
    let (pressed, released, position) = ctx.input(|input| {
        (
            input.pointer.button_pressed(PointerButton::Primary),
            input.pointer.button_released(PointerButton::Primary),
            input.pointer.interact_pos(),
        )
    });

    ctx.data_mut(|data| {
        let state = data.get_temp_mut_or_default::<State>(id);

        if pressed {
            state.pressed = true;
        }
        if let Some(position) = position {
            state.last_pos = Some(position);
        }

        if released && state.pressed {
            state.pressed = false;
            return state.last_pos;
        }

        None
    })
}
