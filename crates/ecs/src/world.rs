use crate::{
	error::Result,
	resource::ResourceMap,
	vec::{error::HandleNotFoundError, GenerationalVec, Handle, HandleAllocator, SlotVec},
};
use std::{
	any::TypeId,
	cell::{Ref, RefCell, RefMut},
	collections::HashMap,
	ops::Deref,
	rc::Rc,
};

/*
	Entities:                    Entity 0                       Entity 1   Entity 2                         Entity 3
	Physics Components   -> Vec( Some(Physics { vel: 3 }),      None,      None,                            Some(Physics { vel: 04 }) )
	Position Components  -> Vec( Some(Position { x: 3, y: 3 }), None,      Some(Position { x: 10, y: -2 }), Some(Position { x: 100, y: -20 }) )
*/
pub type ComponentMap = HashMap<TypeId, ComponentVecHandle>;

pub type Entity = Handle;
pub type ComponentVecHandle = Rc<RefCell<ComponentVec>>;
pub type Component = Box<dyn std::any::Any + 'static>;
pub type ComponentVec = GenerationalVec<Component>;

impl Default for ComponentVec {
	fn default() -> Self {
		GenerationalVec::new(SlotVec::<Component>::default())
	}
}

#[macro_export]
macro_rules! component_vec {
    () => {
        {
			use std::{rc::Rc, cell::RefCell};
			use $crate::world::ComponentVec;
            Rc::new(RefCell::new(ComponentVec::new(vec![])))
        }
    };

    ($($component:expr),*) => {
        {
			use std::{rc::Rc, cell::RefCell};
			use $crate::world::ComponentVec;
            Rc::new(RefCell::new(ComponentVec::new(vec![$(Some($crate::vec::Slot::new(Box::new($component), 0)),)*])))
        }
    }
}

// from itertools
#[macro_export]
macro_rules! izip {
    // @closure creates a tuple-flattening closure for .map() call. usage:
    // @closure partial_pattern => partial_tuple , rest , of , iterators
    // eg. izip!( @closure ((a, b), c) => (a, b, c) , dd , ee )
    ( @closure $p:pat => $tup:expr ) => {
        |$p| $tup
    };

    // The "b" identifier is a different identifier on each recursion level thanks to hygiene.
    ( @closure $p:pat => ( $($tup:tt)* ) , $_iter:expr $( , $tail:expr )* ) => {
        $crate::izip!(@closure ($p, b) => ( $($tup)*, b ) $( , $tail )*)
    };

    // unary
    ($first:expr $(,)*) => {
        std::iter::IntoIterator::into_iter($first)
    };

    // binary
    ($first:expr, $second:expr $(,)*) => {
        $crate::izip!($first)
            .zip($second)
    };

    // n-ary where n > 2
    ( $first:expr $( , $rest:expr )* $(,)* ) => {
        $crate::izip!($first)
            $(
                .zip($rest)
            )*
            .map(
                $crate::izip!(@closure a => (a) $( , $rest )*)
            )
    };
}

// TODO: make systems accessing unregistered components recoverable (maybe by auto registering
// types)
#[macro_export]
macro_rules! system {
	($fn:tt, [$resources:ident, $entity:ident], ($($arg:ident: $arg_type:ty),*), ($component_name:ident: $component_type:ty) -> $result:ty {$($body:tt)*}) => {
		pub fn $fn($($arg: $arg_type,)* world: &mut World) -> $result {
			world
				.get_component_vec_mut::<$component_type>()
				.unwrap_or_else(|| panic!("System accessed an unregistered component type: {:?}", stringify!($component_type)))
				.iter_mut()
				.enumerate()
				.filter_map(|(entity, $component_name)| match ($component_name) {
					Some($component_name) => {
						let $component_name = $component_name.downcast_mut::<$component_type>().unwrap();
						Some((world.resources().clone(), entity, $component_name))
					},
					_ => None,
				})
				.try_for_each(|($resources, $entity, mut $component_name)| {
					$($body)*
				})
		}
    };

    ($fn:tt, [$resources:ident, $entity:ident], ($($arg:ident: $arg_type:ty),*), ($($component_name:ident: $component_type:ty),*) -> $result:ty {$($body:tt)*}) => {
		pub fn $fn($($arg: $arg_type,)* world: &mut World) -> $result {
			izip!(
				$(
					world.get_component_vec_mut::<$component_type>().unwrap_or_else(|| panic!("System accessed an unregistered component type: {:?}", stringify!($component_type))).iter_mut()
				),*
			)
			.enumerate()
			.filter_map(|(entity, ($($component_name),*))| match ($($component_name,)*) {
				($(Some($component_name),)*) => {
					$(
						let $component_name = $component_name.downcast_mut::<$component_type>().unwrap();
					)*
					Some((world.resources().clone(), entity, $( $component_name,)*))
				},
				_ => None,
			})
			.try_for_each(|($resources, $entity, $(mut $component_name,)*)| {
				$($body)*
			})
		}
    }
}

#[derive(Default)]
pub struct World {
	resources: Rc<RefCell<ResourceMap>>,
	components: ComponentMap,
	allocator: HandleAllocator,
}

impl World {
	pub fn new() -> Self {
		Self::default()
	}

	pub const fn resources(&self) -> &Rc<RefCell<ResourceMap>> {
		&self.resources
	}

	pub fn create_entity(&mut self) -> Entity {
		self.create_entities(1)[0]
	}

	pub fn create_entities(&mut self, count: usize) -> Vec<Entity> {
		(0..count).map(|_index| self.allocator.allocate()).collect()
	}

	pub fn remove_entity(&mut self, entity: Entity) {
		self.remove_entities(&[entity]);
	}

	pub fn remove_entities(&mut self, entities: &[Entity]) {
		entities
			.iter()
			.for_each(|entity| self.allocator.deallocate(entity))
	}

	pub fn add_component<T: 'static>(&mut self, entity: Entity, component: T) -> Result<()> {
		self.assign_component::<T>(entity, Some(Box::new(component)))
	}

	pub fn has_component<T: 'static>(&mut self, entity: Entity) -> bool {
		self.get_component::<T>(entity).is_some()
	}

	pub fn remove_component<T: 'static>(&mut self, entity: Entity) -> Result<()> {
		self.assign_component::<T>(entity, None)
	}

	fn assign_component<T: 'static>(
		&mut self,
		entity: Entity,
		value: Option<Component>,
	) -> Result<()> {
		if !self.allocator.handle_exists(&entity) {
			return Err(
				Box::new(HandleNotFoundError { handle: entity }) as Box<dyn std::error::Error>
			);
		}

		let mut components = self
			.components
			.entry(TypeId::of::<T>())
			.or_insert_with(|| Rc::new(RefCell::new(ComponentVec::default())))
			.borrow_mut();

		match value {
			Some(component) => {
				components.insert(entity, component)?;
			}
			None => {
				components.remove(entity);
			}
		}

		Ok(())
	}

	#[must_use]
	pub fn get_component<T: 'static>(&self, entity: Entity) -> Option<Ref<T>> {
		if !self.entity_exists(entity) {
			return None;
		}
		self.components
			.get(&TypeId::of::<T>())
			.and_then(|component_vec| {
				if !entity_has_component(entity, component_vec) {
					return None;
				}
				Some(Ref::map(component_vec.borrow(), |t| {
					t.get(entity)
						.and_then(|component| component.downcast_ref::<T>())
						.unwrap()
				}))
			})
	}

	#[must_use]
	pub fn get_component_mut<T: 'static>(&self, entity: Entity) -> Option<RefMut<T>> {
		if !self.entity_exists(entity) {
			return None;
		}
		self.components
			.get(&TypeId::of::<T>())
			.and_then(|component_vec| {
				if !entity_has_component(entity, component_vec) {
					return None;
				}
				Some(RefMut::map(component_vec.borrow_mut(), |t| {
					t.get_mut(entity)
						.and_then(|c| c.downcast_mut::<T>())
						.unwrap()
				}))
			})
	}

	pub fn get_component_vec<T: 'static>(&self) -> Option<Ref<ComponentVec>> {
		self.components
			.get(&TypeId::of::<T>())
			.map(|component_vec| component_vec.deref().borrow())
	}

	pub fn get_component_vec_mut<T: 'static>(&self) -> Option<RefMut<ComponentVec>> {
		self.components
			.get(&TypeId::of::<T>())
			.map(|component_vec| component_vec.deref().borrow_mut())
	}

	pub fn register_component<T: 'static>(&mut self) {
		self.components
			.entry(TypeId::of::<T>())
			.or_insert(component_vec!());
	}

	pub fn entity_exists(&self, entity: Entity) -> bool {
		self.allocator.is_allocated(&entity)
	}
}

pub fn entity_has_component(entity: Entity, components: &ComponentVecHandle) -> bool {
	components.borrow().get(entity).is_some()
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::ops::DerefMut;

	#[derive(Debug, Default, PartialEq, Copy, Clone)]
	pub struct Position {
		x: f32,
		y: f32,
	}

	#[derive(Debug, Default, PartialEq, Eq, Copy, Clone)]
	pub struct Health {
		value: u8,
	}

	struct Name(String);

	// Translate only named entities
	system!(translation_system, [_resources, _entity], (value: f32), (position: Position, _name: Name, _health: Health) -> Result<()> {
		position.x += value;
		position.y += value;
		Ok(())
	});

	#[derive(Debug, PartialEq)]
	struct DeltaTime(f32);

	// This runs for each entity but ensures we can access and mutate resources from systems
	system!(resource_system, [resources, _entity], (value: f32), (_position: Position) -> Result<()> {
		resources.borrow_mut().insert(DeltaTime(value));
		Ok(())
	});

	#[test]
	fn entity() -> Result<()> {
		let mut world = World::default();
		let entity = world.create_entity();
		world.add_component(entity, Position::default())?;
		assert!(world.get_component::<Position>(entity).is_some());
		world.remove_entity(entity);
		assert!(world.get_component::<Position>(entity).is_none());
		Ok(())
	}

	#[test]
	fn add_component() -> Result<()> {
		let mut world = World::default();
		let entity = world.create_entity();
		assert!(world.get_component::<Position>(entity).is_none());
		assert!(world.get_component::<Health>(entity).is_none());
		world.add_component(entity, Position::default())?;
		world.add_component(entity, Health { value: 10 })?;
		world.get_component_mut::<Health>(entity).unwrap().value = 0;
		assert_eq!(
			world.get_component::<Position>(entity).as_deref(),
			Some(&Position::default())
		);
		assert_eq!(
			world.get_component::<Health>(entity).as_deref(),
			Some(&Health { value: 0 })
		);
		Ok(())
	}

	#[test]
	fn remove_component() -> Result<()> {
		let mut world = World::new();
		let entity = world.create_entity();
		let position = Position { x: 10.0, y: 10.0 };
		world.add_component(entity, position)?;
		assert_eq!(
			world.get_component::<Position>(entity).as_deref(),
			Some(&position)
		);
		world.remove_component::<Position>(entity)?;
		assert!(world.get_component::<Position>(entity).is_none());
		Ok(())
	}

	#[test]
	fn get_component() -> Result<()> {
		let mut world = World::default();
		let entity = world.create_entity();
		world.add_component(entity, Position::default())?;
		assert!(world.has_component::<Position>(entity));
		assert_eq!(
			world.get_component::<Position>(entity).as_deref(),
			Some(&Position::default())
		);
		Ok(())
	}

	#[test]
	fn get_component_mut() -> Result<()> {
		let mut world = World::default();
		let entity = world.create_entity();
		world.add_component(entity, Position::default())?;
		world
			.get_component_mut::<Position>(entity)
			.unwrap()
			.deref_mut()
			.x = 10.0;
		assert_eq!(
			world.get_component::<Position>(entity).as_deref(),
			Some(&Position { x: 10.0, y: 0.0 })
		);
		Ok(())
	}

	#[test]
	fn system() -> Result<()> {
		let mut world = World::default();
		let entity = world.create_entity();
		world.add_component(entity, Position::default())?;
		world.add_component(entity, Health::default())?;
		world.add_component(entity, Name("Tyrell Wellick".to_string()))?;

		translation_system(10.0, &mut world)?;

		assert_eq!(
			world.get_component::<Position>(entity).as_deref(),
			Some(&Position { x: 10.0, y: 10.0 })
		);

		Ok(())
	}

	#[test]
	fn component_exists() -> Result<()> {
		let mut entity_allocator = HandleAllocator::new();
		let entity = entity_allocator.allocate();

		let components = component_vec!();
		components
			.borrow_mut()
			.insert(entity, Box::new(Name("Elliot Alderson".to_string())))?;

		assert!(entity_has_component(entity, &components));

		Ok(())
	}

	#[test]
	fn system_resources() -> Result<()> {
		let mut world = World::default();

		let entity = world.create_entity();
		world.add_component(entity, Position::default())?;

		let value = 0.18;
		resource_system(value, &mut world)?;

		assert_eq!(
			world.resources().borrow().get::<DeltaTime>(),
			Some(&DeltaTime(value))
		);

		Ok(())
	}

	#[test]
	fn resources() -> Result<()> {
		let world = World::default();
		let value = 0.18;
		world.resources.borrow_mut().insert(DeltaTime(value));
		assert_eq!(
			world.resources().borrow().get::<DeltaTime>(),
			Some(&DeltaTime(value))
		);
		Ok(())
	}

	#[test]
	#[should_panic]
	fn unregistered_component() {
		World::default()
			.get_component_vec_mut::<Position>()
			.unwrap();
	}

	#[test]
	#[should_panic]
	fn system_accessed_unregistered_component() {
		let mut world = World::new();
		translation_system(0.14, &mut world).unwrap();
	}

	#[test]
	fn component_registration() -> Result<()> {
		let mut world = World::default();

		assert!(world.get_component_vec_mut::<Position>().is_none());

		let entity = world.create_entity();
		world.add_component(entity, Position::default())?;

		assert!(world.get_component_vec_mut::<Position>().is_some());

		Ok(())
	}
}
