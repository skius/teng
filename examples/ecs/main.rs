//! A simple example of an ECS (Entity-Component-System) in teng.
//!
//! Arbitrary structs can be registered and used as components, and systems are just
//! types that implement `teng::Component`.

use anymap::AnyMap;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::io;
use std::io::stdout;
use teng::components::Component as TengComponent;
use teng::rendering::pixel::Pixel;
use teng::rendering::renderer::Renderer;
use teng::{BreakingAction, Game, SetupInfo, SharedState};

/// An ECS-component that holds the position of an entity.
struct Position {
    x: usize,
    y: usize,
}

/// An ECS-component that holds the display character of an entity.
struct Draw {
    ch: char,
}

/// An ECS-entity.
#[derive(Hash, Eq, PartialEq, Clone, Copy)]
struct Entity(usize);

/// An ECS-system that draws entities with a `Position` and `Draw` component.
struct DrawSystem;

impl TengComponent<Ecs> for DrawSystem {
    fn render(
        &self,
        renderer: &mut dyn Renderer,
        shared_state: &SharedState<Ecs>,
        depth_base: i32,
    ) {
        let ecs = &shared_state.custom;
        for (position, draw) in ecs.entities.iter().filter_map(|entity| {
            let position = ecs.get_component::<Position>(*entity)?;
            let draw = ecs.get_component::<Draw>(*entity)?;
            Some((position, draw))
        }) {
            renderer.render_pixel(position.x, position.y, Pixel::new(draw.ch), depth_base);
        }
    }
}

/// An ECS-system that applies rudimentary physics to entities with a `Position` component.
struct PhysicsSystem;

impl TengComponent<Ecs> for PhysicsSystem {
    fn update(&mut self, _update_info: teng::UpdateInfo, shared_state: &mut SharedState<Ecs>) {
        let ecs = &mut shared_state.custom;
        for &entity in &ecs.entities {
            let Some(position) = ecs.components.get_mut_from_entity::<Position>(entity) else {
                continue;
            };
            position.y += 1;
            if position.y >= shared_state.display_info.height() {
                position.y = 0;
            }
        }
    }
}

fn main() -> io::Result<()> {
    teng::terminal_setup()?;
    teng::install_panic_handler();

    let mut game = Game::new(stdout());
    game.install_recommended_components();
    game.add_component(Box::new(EcsComponent::default()));
    game.add_component(Box::new(DrawSystem));
    game.add_component(Box::new(PhysicsSystem));
    game.run()?;

    teng::terminal_cleanup()?;
    Ok(())
}

struct ComponentList {
    inner: AnyMap,
}

impl Default for ComponentList {
    fn default() -> Self {
        Self::new()
    }
}

impl ComponentList {
    fn new() -> Self {
        Self {
            inner: AnyMap::new(),
        }
    }

    fn add_to_entity<T: 'static>(&mut self, entity: Entity, component: T) {
        let map = self.get_mut::<T>().expect("Component not registered");
        map.insert(entity, component);
    }

    fn get_from_entity<T: 'static>(&self, entity: Entity) -> Option<&T> {
        let map = self.get::<T>()?;
        map.get(&entity)
    }

    fn get_mut_from_entity<T: 'static>(&mut self, entity: Entity) -> Option<&mut T> {
        let map = self.get_mut::<T>()?;
        map.get_mut(&entity)
    }

    fn get<T: 'static>(&self) -> Option<&HashMap<Entity, T>> {
        self.inner.get::<HashMap<Entity, T>>()
    }

    fn get_mut<T: 'static>(&mut self) -> Option<&mut HashMap<Entity, T>> {
        self.inner.get_mut::<HashMap<Entity, T>>()
    }

    fn register<T: 'static>(&mut self) {
        self.inner.insert::<HashMap<Entity, T>>(HashMap::new());
    }
}

/// Shared state for ECS.
#[derive(Default)]
struct Ecs {
    entities: Vec<Entity>,
    max_key: usize,
    components: ComponentList,
}

impl Ecs {
    fn new() -> Self {
        Self {
            entities: Vec::new(),
            max_key: 0,
            components: ComponentList::new(),
        }
    }

    fn create_entity(&mut self) -> Entity {
        let entity = Entity(self.max_key);
        self.entities.push(entity);
        self.max_key += 1;
        entity
    }

    fn add_component<T: 'static>(&mut self, entity: Entity, component: T) {
        self.components.add_to_entity(entity, component);
    }

    fn get_component<T: 'static>(&self, entity: Entity) -> Option<&T> {
        self.components.get_from_entity(entity)
    }

    fn get_mut_component<T: 'static>(&mut self, entity: Entity) -> Option<&mut T> {
        self.components.get_mut_from_entity(entity)
    }
}

/// A wrapper component that sets up the ECS and creates new entities.
#[derive(Default)]
struct EcsComponent {
    width: usize,
    height: usize,
}

impl TengComponent<Ecs> for EcsComponent {
    fn setup(&mut self, setup_info: &SetupInfo, shared_state: &mut SharedState<Ecs>) {
        self.width = setup_info.display_info.width();
        self.height = setup_info.display_info.height();

        let ecs = &mut shared_state.custom;
        ecs.components.register::<Position>();
        ecs.components.register::<Draw>();
    }

    fn on_event(
        &mut self,
        event: Event,
        shared_state: &mut SharedState<Ecs>,
    ) -> Option<BreakingAction> {
        let ecs = &mut shared_state.custom;
        if let Event::Key(KeyEvent {
            kind: KeyEventKind::Press,
            code: KeyCode::Char(ch),
            ..
        }) = event
        {
            // Create a new entity with a random position and the pressed key as display character.
            let entity = ecs.create_entity();
            let x = rand::random::<usize>() % self.width;
            let y = rand::random::<usize>() % self.height;
            ecs.add_component(entity, Position { x, y });
            ecs.add_component(entity, Draw { ch });
        }

        None
    }
}
