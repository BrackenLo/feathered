//====================================================================

use shipyard::Component;

//====================================================================

#[derive(Component)]
pub struct BuiltSimpleCollision {
    start_x: f32,
    end_x: f32,
    start_y: f32,
    end_y: f32,
    start_z: f32,
    end_z: f32,
}

//====================================================================
