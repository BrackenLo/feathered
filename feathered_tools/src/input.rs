//====================================================================

use std::{collections::HashSet, hash::Hash};

use feathered_common::WindowSize;
use feathered_runner::events::{MouseButton, WindowInputEvent};
use feathered_shipyard::{
    builder::{First, Last, Plugin, WorkloadBuilder},
    events::EventHandle,
    Res, ResMut,
};
use shipyard::Unique;

pub use feathered_runner::events::KeyCode;

//====================================================================

pub struct InputPlugin;
impl Plugin for InputPlugin {
    fn build_plugin(self, builder: &mut WorkloadBuilder) {
        builder.add_plugin(KeyboardPlugin).add_plugin(MousePlugin);
    }
}

pub struct KeyboardPlugin;
impl Plugin for KeyboardPlugin {
    fn build_plugin(self, builder: &mut WorkloadBuilder) {
        builder
            .insert(Input::<KeyCode>::default())
            .add_workload(First, sys_process_inputs)
            .add_workload(Last, sys_reset_input::<KeyCode>);
    }
}

pub struct MousePlugin;
impl Plugin for MousePlugin {
    fn build_plugin(self, builder: &mut WorkloadBuilder) {
        builder
            .insert(Input::<MouseButton>::default())
            .insert(MouseInput::default())
            .add_workload(First, sys_process_inputs)
            .add_workload(
                Last,
                (sys_reset_input::<MouseButton>, sys_reset_mouse_input),
            );
    }
}

//====================================================================

fn sys_process_inputs(
    input_event: Res<EventHandle<WindowInputEvent>>,
    size: Res<WindowSize>,

    mut keys: Option<ResMut<Input<KeyCode>>>,
    mut mouse_buttons: Option<ResMut<Input<MouseButton>>>,
    mut mouse_input: Option<ResMut<MouseInput>>,
) {
    input_event.iter().for_each(|event| match event {
        WindowInputEvent::KeyInput { key, pressed } => match (&mut keys, pressed) {
            (Some(keys), true) => keys.add_pressed(*key),
            (Some(keys), false) => keys.remove_pressed(*key),
            _ => {}
        },

        WindowInputEvent::MouseInput { button, pressed } => match (&mut mouse_buttons, pressed) {
            (Some(buttons), true) => buttons.add_pressed(*button),
            (Some(buttons), false) => buttons.remove_pressed(*button),
            _ => {}
        },

        WindowInputEvent::CursorMoved { position } => match &mut mouse_input {
            Some(mouse) => {
                mouse.position = glam::vec2(position.0 as f32, position.1 as f32);
                mouse.screen_position =
                    glam::vec2(mouse.position.x, size.height_f32() - mouse.position.y);
            }
            None => {}
        },

        WindowInputEvent::MouseWheel { delta } => match &mut mouse_input {
            Some(mouse) => mouse.scroll = delta.clone().into(),
            None => {}
        },

        WindowInputEvent::CursorMotion { delta } => match &mut mouse_input {
            Some(mouse) => mouse.position_delta += glam::vec2(delta.0 as f32, delta.1 as f32),
            None => {}
        },
    });
}

//====================================================================

#[derive(Unique, Debug)]
pub struct Input<T>
where
    T: 'static + Send + Sync + Eq + PartialEq + Hash + Clone + Copy,
{
    pressed: HashSet<T>,
    just_pressed: HashSet<T>,
    released: HashSet<T>,
}

impl<T> Default for Input<T>
where
    T: 'static + Send + Sync + Eq + PartialEq + Hash + Clone + Copy,
{
    fn default() -> Self {
        Self {
            pressed: HashSet::new(),
            just_pressed: HashSet::new(),
            released: HashSet::new(),
        }
    }
}

#[allow(dead_code)]
impl<T> Input<T>
where
    T: 'static + Send + Sync + Eq + PartialEq + Hash + Clone + Copy,
{
    fn add_pressed(&mut self, input: T) {
        self.pressed.insert(input);
        self.just_pressed.insert(input);
    }

    fn remove_pressed(&mut self, input: T) {
        self.pressed.remove(&input);
        self.released.insert(input);
    }

    fn reset(&mut self) {
        self.just_pressed.clear();
        self.released.clear();
    }

    #[inline]
    pub fn pressed(&self, input: T) -> bool {
        self.pressed.contains(&input)
    }

    #[inline]
    pub fn just_pressed(&self, input: T) -> bool {
        self.just_pressed.contains(&input)
    }

    #[inline]
    pub fn _released(&self, input: T) -> bool {
        self.released.contains(&input)
    }
}

fn sys_reset_input<T>(mut input: ResMut<Input<T>>)
where
    T: 'static + Send + Sync + Eq + PartialEq + Hash + Clone + Copy,
{
    input.reset();
}

//====================================================================

#[derive(Unique, Debug, Default)]
pub struct MouseInput {
    position: glam::Vec2,
    screen_position: glam::Vec2,
    position_delta: glam::Vec2,
    scroll: glam::Vec2,
}

impl MouseInput {
    #[inline]
    pub fn position(&self) -> glam::Vec2 {
        self.position
    }

    #[inline]
    pub fn screen_position(&self) -> glam::Vec2 {
        self.screen_position
    }

    #[inline]
    pub fn position_delta(&self) -> glam::Vec2 {
        self.position_delta
    }

    #[inline]
    pub fn scroll(&self) -> glam::Vec2 {
        self.scroll
    }
}

fn sys_reset_mouse_input(mut mouse: ResMut<MouseInput>) {
    mouse.position_delta = glam::Vec2::ZERO;
    mouse.scroll = glam::Vec2::ZERO;
}

//====================================================================
