//====================================================================

use std::{fmt::Display, sync::Arc};

use feathered_shipyard::{
    events::{Event, EventBuilder},
    prelude::*,
};
use shipyard::Unique;
use window_handles::WindowHandle;

mod window_handles;

pub use web_time::{Duration, Instant};

//====================================================================

pub struct CommonPlugin;
impl Plugin for CommonPlugin {
    fn build_plugin(self, builder: &mut WorkloadBuilder) {
        builder
            .insert(Time::default())
            .register_event::<WindowResizeEvent>()
            .add_workload(First, sys_update_time);
    }
}

//====================================================================

#[cfg(not(target_arch = "wasm32"))]
#[derive(Debug)]
pub struct WasmWrapper<T>(T);

#[cfg(target_arch = "wasm32")]
#[derive(Debug)]
pub struct WasmWrapper<T>(send_wrapper::SendWrapper<T>);

impl<T> WasmWrapper<T> {
    #[inline]
    pub fn new(data: T) -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        return Self(data);

        #[cfg(target_arch = "wasm32")]
        return Self(send_wrapper::SendWrapper::new(data));
    }

    #[inline]
    pub fn inner(&self) -> &T {
        &self.0
    }

    #[inline]
    pub fn inner_mut(&mut self) -> &mut T {
        &mut self.0
    }

    #[inline]
    pub fn take(self) -> T {
        #[cfg(not(target_arch = "wasm32"))]
        return self.0;

        #[cfg(target_arch = "wasm32")]
        return self.0.take();
    }
}

//====================================================================

#[derive(Clone, Copy, Debug, Hash, PartialEq)]
pub struct Size<T> {
    pub width: T,
    pub height: T,
}

impl<T> Size<T> {
    #[inline]
    pub fn new(width: T, height: T) -> Self {
        Self { width, height }
    }
}

impl<T> From<(T, T)> for Size<T> {
    #[inline]
    fn from(value: (T, T)) -> Self {
        Self {
            width: value.0,
            height: value.1,
        }
    }
}

impl<T> From<Size<T>> for (T, T) {
    #[inline]
    fn from(value: Size<T>) -> Self {
        (value.width, value.height)
    }
}

impl<T: Display> Display for Size<T> {
    #[inline]
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({}, {})", self.width, self.height)
    }
}

//====================================================================

#[derive(Unique)]
pub struct Time {
    elapsed: Instant,

    last_frame: Instant,
    delta: Duration,
    delta_seconds: f32,
}

impl Default for Time {
    fn default() -> Self {
        Self {
            elapsed: Instant::now(),
            last_frame: Instant::now(),
            delta: Duration::ZERO,
            delta_seconds: 0.,
        }
    }
}

#[allow(dead_code)]
impl Time {
    #[inline]
    pub fn elapsed(&self) -> &Instant {
        &self.elapsed
    }

    #[inline]
    pub fn delta(&self) -> &Duration {
        &self.delta
    }

    #[inline]
    pub fn delta_seconds(&self) -> f32 {
        self.delta_seconds
    }
}

pub fn sys_update_time(mut time: ResMut<Time>) {
    time.delta = time.last_frame.elapsed();
    time.delta_seconds = time.delta.as_secs_f32();

    time.last_frame = Instant::now();
}

//====================================================================

#[derive(Unique)]
pub struct WindowRaw {
    window: WasmWrapper<Arc<dyn WindowHandle>>,
    size: Size<u32>,
}

impl WindowRaw {
    #[inline]
    pub fn new(window: Arc<dyn WindowHandle>, size: Size<u32>) -> Self {
        Self {
            window: WasmWrapper::new(window),
            size,
        }
    }

    #[inline]
    pub fn arc(&self) -> &Arc<dyn WindowHandle> {
        &self.window.0
    }

    #[inline]
    pub fn size(&self) -> Size<u32> {
        self.size
    }
}

//====================================================================

#[derive(Unique)]
pub struct WindowSize(Size<u32>);

impl WindowSize {
    #[inline]
    pub fn new(size: Size<u32>) -> Self {
        Self(size)
    }

    #[inline]
    pub fn size(&self) -> Size<u32> {
        self.0
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.0.width
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.0.height
    }

    #[inline]
    pub fn width_f32(&self) -> f32 {
        self.0.width as f32
    }

    #[inline]
    pub fn height_f32(&self) -> f32 {
        self.0.height as f32
    }
}

#[derive(Event, Debug)]
pub struct WindowResizeEvent(Size<u32>);

impl WindowResizeEvent {
    #[inline]
    pub fn new(new_size: Size<u32>) -> Self {
        Self(new_size)
    }

    #[inline]
    pub fn size(&self) -> Size<u32> {
        self.0
    }
}

//====================================================================
