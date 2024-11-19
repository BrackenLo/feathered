//====================================================================

pub mod builder;
pub mod events;
pub mod runner;
pub mod tools;

//====================================================================

pub mod prelude {
    pub use crate::{
        builder::{
            First, Last, Plugin, Render, RenderPrep, Setup, SubStages, Update, WorkloadBuilder,
        },
        tools::UniqueTools,
        Res, ResMut,
    };
    pub use shipyard::{View, ViewMut};
}

//====================================================================

pub type Res<'a, T> = shipyard::UniqueView<'a, T>;
pub type ResMut<'a, T> = shipyard::UniqueViewMut<'a, T>;

//====================================================================
