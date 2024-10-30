//====================================================================

use std::{sync::Arc, time::Duration};

use events::WindowInputEvent;
use feathered_common::Size;
use feathered_shipyard::{
    builder::{register_main_stages, WorkloadBuilder},
    events::EventBuilder,
    runner::WorkloadRunner,
    tools::UniqueTools,
};
use shipyard::Unique;
use winit::{
    application::ApplicationHandler,
    event::{DeviceEvent, DeviceId, StartCause, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    window::{WindowAttributes, WindowId},
};

pub mod events;
pub mod window;

//====================================================================

enum RunnerInnerState {
    Waiting(Option<(shipyard::World, WorkloadRunner)>),
    Running(RunnerInner),
}

pub struct Runner(RunnerInnerState);

impl Runner {
    pub fn run<F>(build_app: F)
    where
        F: FnOnce(&mut WorkloadBuilder),
    {
        let world = shipyard::World::new();
        let mut builder = WorkloadBuilder::new(&world);

        register_main_stages(&mut builder);
        builder.register_event::<WindowInputEvent>();

        build_app(&mut builder);
        let runner = builder.build();

        let mut runner = Self(RunnerInnerState::Waiting(Some((world, runner))));

        let event_loop = EventLoop::new().unwrap();
        event_loop.run_app(&mut runner).unwrap();
    }
}

impl ApplicationHandler for Runner {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        log::trace!("App Resumed - Creating inner app");

        let (world, workload_runner) = match &mut self.0 {
            RunnerInnerState::Waiting(inner) => inner.take().unwrap(),
            RunnerInnerState::Running(..) => {
                log::warn!("Application resumed again...");
                return;
            }
        };

        let inner = RunnerInner::new(event_loop, world, workload_runner);
        self.0 = RunnerInnerState::Running(inner);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if let RunnerInnerState::Running(inner) = &mut self.0 {
            inner.window_event(event_loop, window_id, event);
        };
    }

    fn new_events(&mut self, _event_loop: &ActiveEventLoop, cause: StartCause) {
        if let RunnerInnerState::Running(inner) = &mut self.0 {
            if let StartCause::ResumeTimeReached { .. } = cause {
                inner.resumed()
            }
        }
    }

    // TODO
    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: ()) {
        let _ = (event_loop, event);
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if let RunnerInnerState::Running(inner) = &mut self.0 {
            inner.device_event(event_loop, device_id, event);
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
    }

    fn exiting(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
    }

    fn memory_warning(&mut self, event_loop: &ActiveEventLoop) {
        let _ = event_loop;
    }
}

//====================================================================

const DEFAULT_TIMESTEP: f32 = 1. / 75.;

#[derive(Unique)]
pub struct RunnerTargetFPS(Duration);

impl RunnerTargetFPS {
    pub fn update_target(&mut self, duration: Duration) {
        // if duration.as_secs_f32() >= 1. {
        //     return;
        // }
        self.0 = duration;
    }
}

impl Default for RunnerTargetFPS {
    fn default() -> Self {
        Self(Duration::from_secs_f32(DEFAULT_TIMESTEP))
    }
}

//====================================================================

struct RunnerInner {
    world: shipyard::World,
    workload_runner: WorkloadRunner,
}

impl RunnerInner {
    fn new(
        event_loop: &ActiveEventLoop,
        world: shipyard::World,
        workload_runner: WorkloadRunner,
    ) -> Self {
        let window = Arc::new(
            event_loop
                .create_window(WindowAttributes::default())
                .unwrap(),
        );

        world.run_with_data(window::sys_add_window, window);
        world.insert(RunnerTargetFPS::default());
        workload_runner.prep(&world);

        Self {
            world,
            workload_runner,
        }
    }

    // TODO
    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::Resized(new_size) => {
                self.resize(Size::new(new_size.width, new_size.height))
            }

            WindowEvent::Destroyed => log::error!("Window was destroyed"), // panic!("Window was destroyed"),
            WindowEvent::CloseRequested => {
                log::info!("Close requested. Closing App.");
                event_loop.exit();
            }

            WindowEvent::RedrawRequested => {
                self.tick();

                let timestep = self
                    .world
                    .borrow::<feathered_shipyard::Res<RunnerTargetFPS>>()
                    .unwrap();

                event_loop
                    .set_control_flow(winit::event_loop::ControlFlow::wait_duration(timestep.0));
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if let winit::keyboard::PhysicalKey::Code(key) = event.physical_key {
                    self.world.run_with_data(
                        events::sys_send_event,
                        WindowInputEvent::KeyInput {
                            key,
                            pressed: event.state.is_pressed(),
                        },
                    )
                }
            }

            WindowEvent::MouseInput { state, button, .. } => self.world.run_with_data(
                events::sys_send_event,
                WindowInputEvent::MouseInput {
                    button,
                    pressed: state.is_pressed(),
                },
            ),

            WindowEvent::CursorMoved { position, .. } => self.world.run_with_data(
                events::sys_send_event,
                WindowInputEvent::CursorMoved {
                    position: position.into(),
                },
            ),

            WindowEvent::MouseWheel { delta, .. } => match delta {
                winit::event::MouseScrollDelta::LineDelta(h, v) => {
                    self.world.run_with_data(
                        events::sys_send_event,
                        WindowInputEvent::MouseWheel { delta: (h, v) },
                    );
                }
                winit::event::MouseScrollDelta::PixelDelta(_) => {}
            },

            _ => {}
        }
    }

    fn resumed(&mut self) {
        self.world
            .run(|window: shipyard::UniqueView<window::Window>| window.request_redraw());
    }

    // TODO
    fn device_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        match event {
            DeviceEvent::MouseMotion { delta } => self.world.run_with_data(
                events::sys_send_event,
                WindowInputEvent::CursorMotion { delta },
            ),
            _ => {}
        }
    }
}

impl RunnerInner {
    fn resize(&mut self, new_size: Size<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            log::warn!("Resize width or height of '0' provided");
            return;
        }

        self.world.run_with_data(window::sys_resize, new_size);
    }

    fn tick(&mut self) {
        self.workload_runner.run(&self.world);
    }
}

//====================================================================
