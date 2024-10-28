//====================================================================

use shipyard::{IntoWorkload, Unique};

use crate::{
    builder::{First, SubStages, WorkloadBuilder},
    ResMut,
};

//====================================================================

pub use feathered_proc::Event;
pub trait Event: 'static + Send + Sync + std::fmt::Debug {}

pub trait EventBuilder {
    // fn add_event<E: Event>(
    //     &mut self,
    //     event: E,
    //     substage: SubStages,
    //     workload: shipyard::Workload,
    // ) -> &mut Self;

    fn register_event<E: Event>(&mut self) -> &mut Self;
}

impl<'a> EventBuilder for WorkloadBuilder<'a> {
    // fn add_event<E: Event>(
    //     &mut self,
    //     event: E,
    //     substage: SubStages,
    //     workload: shipyard::Workload,
    // ) -> &mut Self {
    //     self.get_inner().log(format!(
    //         "Adding workload for event '{}'",
    //         std::any::type_name::<E>()
    //     ));

    //     self.get_inner().add_workload_sub(event, substage, workload);
    //     self
    // }

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
    fn setup_events(&mut self) {
        std::mem::swap(&mut self.pending_events, &mut self.events);
        self.pending_events.clear();
    }
}

fn sys_setup_events<E: Event>(mut handle: ResMut<EventHandle<E>>) {
    handle.setup_events();
}

//====================================================================
