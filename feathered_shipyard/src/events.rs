//====================================================================

use std::fmt::Debug;

use shipyard::{Borrow, BorrowInfo, IntoWorkload, Unique, WorkloadModificator};

use crate::{
    builder::{First, SubStages, WorkloadBuilder},
    Res, ResMut,
};

//====================================================================

pub use feathered_proc::Event;
pub trait Event: 'static + Send + Sync + std::fmt::Debug {}

pub trait EventBuilder {
    fn register_event<E: Event>(&mut self) -> &mut Self;

    #[inline]
    fn event_workload<E: Event>(
        &mut self,
        workload_id: impl shipyard::Label + Debug,
        workload: shipyard::Workload,
    ) -> &mut Self {
        self.event_workload_sub::<E>(workload_id, SubStages::Main, workload)
    }

    fn event_workload_sub<E: Event>(
        &mut self,
        workload_id: impl shipyard::Label + Debug,
        substage: SubStages,
        workload: shipyard::Workload,
    ) -> &mut Self;
}

impl<'a> EventBuilder for WorkloadBuilder<'a> {
    fn register_event<E: Event>(&mut self) -> &mut Self {
        self.get_inner().log(format!(
            "Registering event type '{}'",
            std::any::type_name::<E>()
        ));

        self.get_world().add_unique(EventHandle::<E> {
            pending_events: Vec::new(),
            events: Vec::new(),
        });

        self.get_inner().add_workload_sub(
            First,
            SubStages::Main,
            (sys_setup_events::<E>).into_workload(),
            true,
        );

        self
    }

    fn event_workload_sub<E: Event>(
        &mut self,
        workload_id: impl shipyard::Label + Debug,
        substage: SubStages,
        workload: shipyard::Workload,
    ) -> &mut Self {
        self.get_inner().log(format!(
            "Adding event workload '{}' for '{:?}' - substage {:?}",
            std::any::type_name::<E>(),
            workload_id,
            substage
        ));

        self.get_inner().add_workload_sub(
            workload_id,
            substage,
            workload.skip_if(sys_check_skip_event::<E>),
            true,
        );

        self
    }
}

//====================================================================

pub trait ReadEvents<E: Event> {
    fn iter(&self) -> std::slice::Iter<E>;
    fn events(&self) -> &Vec<E>;
    fn first(&self) -> Option<&E>;
    fn last(&self) -> Option<&E>;
}

pub trait WriteEvents<E: Event> {
    fn send_event(&mut self, event: E);
}

//--------------------------------------------------

trait GetEventHandle<E: Event> {
    fn handle(&self) -> &EventHandle<E>;
}

trait GetEventHandleMut<E: Event> {
    fn handle_mut(&mut self) -> &mut EventHandle<E>;
}

//--------------------------------------------------

impl<E, T> ReadEvents<E> for T
where
    E: Event,
    T: GetEventHandle<E>,
{
    #[inline]
    fn iter(&self) -> std::slice::Iter<E> {
        self.handle().events.iter()
    }

    #[inline]
    fn events(&self) -> &Vec<E> {
        &self.handle().events
    }

    #[inline]
    fn first(&self) -> Option<&E> {
        self.handle().events.first()
    }

    #[inline]
    fn last(&self) -> Option<&E> {
        self.handle().events.last()
    }
}

impl<E, T> WriteEvents<E> for T
where
    E: Event,
    T: GetEventHandleMut<E>,
{
    #[inline]
    fn send_event(&mut self, event: E) {
        self.handle_mut().pending_events.push(event)
    }
}

//====================================================================

#[derive(Unique)]
pub struct EventHandle<E: Event> {
    pending_events: Vec<E>,
    events: Vec<E>,
}

#[derive(Borrow, BorrowInfo)]
pub struct EventReader<'v, E: Event> {
    handle: Res<'v, EventHandle<E>>,
}

#[derive(Borrow, BorrowInfo)]
pub struct EventSender<'v, E: Event> {
    handle: ResMut<'v, EventHandle<E>>,
}

//--------------------------------------------------

impl<E: Event> GetEventHandle<E> for EventHandle<E> {
    #[inline]
    fn handle(&self) -> &EventHandle<E> {
        self
    }
}
impl<E: Event> GetEventHandleMut<E> for EventHandle<E> {
    #[inline]
    fn handle_mut(&mut self) -> &mut EventHandle<E> {
        self
    }
}

impl<'v, E: Event> GetEventHandle<E> for EventReader<'v, E> {
    #[inline]
    fn handle(&self) -> &EventHandle<E> {
        &self.handle
    }
}

impl<'v, E: Event> GetEventHandle<E> for EventSender<'v, E> {
    #[inline]
    fn handle(&self) -> &EventHandle<E> {
        &self.handle
    }
}
impl<'v, E: Event> GetEventHandleMut<E> for EventSender<'v, E> {
    #[inline]
    fn handle_mut(&mut self) -> &mut EventHandle<E> {
        &mut self.handle
    }
}

//====================================================================

impl<E: Event> EventHandle<E> {
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
