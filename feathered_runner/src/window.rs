//====================================================================

use std::sync::Arc;

use feathered_common::{Size, WindowRaw, WindowResizeEvent, WindowSize};
use feathered_shipyard::{events::EventHandle, tools::UniqueTools, ResMut};
use shipyard::{AllStoragesView, Unique};

//====================================================================

#[derive(Unique)]
pub struct Window(Arc<winit::window::Window>);

impl Window {
    #[inline]
    pub fn inner(&self) -> &winit::window::Window {
        &self.0
    }

    #[inline]
    pub fn request_redraw(&self) {
        self.0.request_redraw();
    }
}

//====================================================================

pub(crate) fn sys_add_window(window: Arc<winit::window::Window>, all_storages: AllStoragesView) {
    let size = Size::new(window.inner_size().width, window.inner_size().height);

    all_storages
        .insert(WindowSize::new(size))
        .insert(Window(window.clone()))
        .insert(WindowRaw::new(window.clone(), size));
}

pub(crate) fn sys_resize(
    new_size: Size<u32>,
    mut window_size: ResMut<WindowSize>,
    mut resize_event: ResMut<EventHandle<WindowResizeEvent>>,
) {
    *window_size = WindowSize::new(new_size);
    resize_event.send_event(WindowResizeEvent::new(new_size));
}

//====================================================================
