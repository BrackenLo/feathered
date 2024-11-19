//====================================================================

use feathered_shipyard::{
    events::{Event, EventHandle, WriteEvents},
    ResMut,
};
pub use winit::{event::MouseButton, keyboard::KeyCode};

//====================================================================

#[derive(Event, Debug)]
pub enum WindowInputEvent {
    KeyInput { key: KeyCode, pressed: bool },
    MouseInput { button: MouseButton, pressed: bool },
    CursorMoved { position: (f64, f64) },
    MouseWheel { delta: (f32, f32) },
    CursorMotion { delta: (f64, f64) },
}

pub(crate) fn sys_send_event<E: Event>(event: E, mut event_handle: ResMut<EventHandle<E>>) {
    event_handle.send_event(event);
}

//====================================================================
