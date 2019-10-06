use amethyst::ecs::{storage::NullStorage, Component};

/// Camera Boom handle tag, used to identify the camera boom handle entity
#[derive(Default)]
pub struct CameraBoomHandle;

impl Component for CameraBoomHandle {
    type Storage = NullStorage<Self>;
}

/// Tag used to identify the character body entity.
#[derive(Default)]
pub struct CharacterBody;

impl Component for CharacterBody {
    type Storage = NullStorage<Self>;
}
