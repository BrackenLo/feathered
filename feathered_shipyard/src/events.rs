//====================================================================

use std::fmt::Debug;

use shipyard::{IntoWorkload, Unique, WorkloadModificator};

use crate::{
    builder::{First, SubStages, WorkloadBuilder},
    Res, ResMut,
};

//====================================================================

pub use feathered_proc::Event;
pub trait Event: 'static + Send + Sync + std::fmt::Debug {}

pub trait EventBuilder {
    fn register_event<E: Event>(&mut self) -> &mut Self;

    fn event_workload<E: Event>(
        &mut self,
        workload_id: impl shipyard::Label + Debug,
        workload: shipyard::Workload,
    ) -> &mut Self;
}

impl<'a> EventBuilder for WorkloadBuilder<'a> {
    fn register_event<E: Event>(&mut self) -> &mut Self {
        self.get_inner().log(format!(
            "Registering event type '{}'",
            std::any::type_name::<E>()
        ));

        self.get_world().add_unique(EventHandle::<E>::default());

        self.get_inner().add_workload_sub(
            First,
            SubStages::Main,
            (sys_setup_events::<E>).into_workload(),
        );

        self
    }

    fn event_workload<E: Event>(
        &mut self,
        workload_id: impl shipyard::Label + Debug,
        workload: shipyard::Workload,
    ) -> &mut Self {
        self.get_inner().add_workload_sub(
            workload_id,
            SubStages::Main,
            workload.skip_if(sys_check_skip_event::<E>),
        );

        self
    }
}

//====================================================================

#[derive(Unique)]
pub struct EventHandle<E: Event> {
    pending_events: Vec<E>,
    events: Vec<E>,
}

impl<E: Event> Default for EventHandle<E> {
    fn default() -> Self {
        Self {
            pending_events: Vec::new(),
            events: Vec::new(),
        }
    }
}

impl<E: Event> EventHandle<E> {
    #[inline]
    pub fn send_event(&mut self, event: E) {
        self.pending_events.push(event);
    }

    #[inline]
    pub fn iter(&self) -> std::slice::Iter<E> {
        self.events.iter()
    }

    #[inline]
    pub fn events(&self) -> &Vec<E> {
        &self.events
    }

    #[inline]
    fn setup_events(&mut self) {
        std::mem::swap(&mut self.pending_events, &mut self.events);
        self.pending_events.clear();
    }
}

#[inline]
fn sys_setup_events<E: Event>(mut handle: ResMut<EventHandle<E>>) {
    handle.setup_events();
}

#[inline]
pub fn sys_check_skip_event<E: Event>(handle: Res<EventHandle<E>>) -> bool {
    handle.events.is_empty()
}

//====================================================================
