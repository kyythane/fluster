use crate::{
    actions::{BoundsKindDefinition, ContainerCreationDefintition, ContainerCreationProperty},
    ecs::resources::QuadTreeLayer,
    engine::Engine,
    types::basic::{ContainerId, LibraryId, ScaleRotationTranslation},
};
use pathfinder_geometry::transform2d::Transform2F;

pub fn new_container(engine: &mut Engine, parent: ContainerId) -> ContainerId {
    let container_id = ContainerId::new();
    engine.create_container(&ContainerCreationDefintition::new(
        parent,
        container_id,
        vec![],
    ));
    container_id
}

pub fn new_positioned_container(
    engine: &mut Engine,
    parent: ContainerId,
    transform: Transform2F,
) -> ContainerId {
    let container_id = ContainerId::new();
    engine.create_container(&ContainerCreationDefintition::new(
        parent,
        container_id,
        vec![ContainerCreationProperty::Transform(
            ScaleRotationTranslation::from_transform(transform),
        )],
    ));
    container_id
}

pub fn new_display_container(
    engine: &mut Engine,
    parent: ContainerId,
    transform: Transform2F,
    library_id: LibraryId,
) -> ContainerId {
    let container_id = ContainerId::new();
    engine.create_container(&ContainerCreationDefintition::new(
        parent,
        container_id,
        vec![
            ContainerCreationProperty::Transform(ScaleRotationTranslation::from_transform(
                transform,
            )),
            ContainerCreationProperty::Display(library_id),
        ],
    ));
    container_id
}

pub fn new_display_container_with_collision(
    engine: &mut Engine,
    parent: ContainerId,
    transform: Transform2F,
    library_id: LibraryId,
    layers: Vec<QuadTreeLayer>,
) -> ContainerId {
    let container_id = ContainerId::new();
    let mut properties = vec![
        ContainerCreationProperty::Transform(ScaleRotationTranslation::from_transform(transform)),
        ContainerCreationProperty::Bounds(BoundsKindDefinition::Display),
        ContainerCreationProperty::Display(library_id),
    ];
    for layer in layers {
        properties.push(ContainerCreationProperty::Layer(layer));
    }
    engine.create_container(&ContainerCreationDefintition::new(
        parent,
        container_id,
        properties,
    ));
    container_id
}
