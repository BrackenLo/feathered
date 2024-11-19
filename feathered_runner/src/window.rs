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
    #[cfg(target_arch = "wasm32")]
    {
        use winit::{dpi::PhysicalSize, platform::web::WindowExtWebSys};

        log::info!("Adding canvas to window");

        if let None = window.request_inner_size(PhysicalSize::new(450, 400)) {
            log::warn!("Wasm Resize Warning: Got none when requesting window inner size");
        }

        web_sys::window()
            .and_then(|win| win.document())
            .and_then(|doc| {
                let dst = doc.get_element_by_id("feathered_app")?;
                let canvas = web_sys::Element::from(window.canvas()?);
                dst.append_child(&canvas).ok()?;
                Some(())
            })
            .expect("Couldn't append canvas to document body.");
    }

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
