//====================================================================

pub mod builder;
pub mod events;
pub mod runner;
pub mod tools;

//====================================================================

pub type Res<'a, T> = shipyard::UniqueView<'a, T>;
pub type ResMut<'a, T> = shipyard::UniqueViewMut<'a, T>;

//====================================================================
