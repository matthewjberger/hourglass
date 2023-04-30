use criterion::{criterion_group, criterion_main, Criterion};
use ecs::{izip, system, world::World};
use std::time::Duration;

fn insertion(c: &mut Criterion) {
	c.bench_function("inserting 1 million entities", |b| {
		let mut world = World::new();
		b.iter(|| world.create_entities(1_000_000))
	});
}

fn removal(c: &mut Criterion) {
	c.bench_function("removing 1 million entities", |b| {
		let mut world = World::new();
		let number_of_entities = 1_000_000;
		let entities = world.create_entities(number_of_entities);
		b.iter(|| world.remove_entities(&entities))
	});
}

fn reinsertion(c: &mut Criterion) {
	c.bench_function("recreating 1 million entities", |b| {
		let mut world = World::new();
		let number_of_entities = 1_000_000;
		let entities = world.create_entities(number_of_entities);
		world.remove_entities(&entities);
		b.iter(|| world.create_entities(number_of_entities))
	});
}

#[derive(Default)]
struct Position(f32, f32);

#[derive(Default)]
struct Health(u8);

#[derive(Default)]
struct Name(String);

fn component_insertion(c: &mut Criterion) {
	c.bench_function("inserting 1 million components", |b| {
		let mut world = World::new();
		let number_of_entities = 1_000_000;
		let entities = world.create_entities(number_of_entities);
		world.remove_entities(&entities);
		b.iter(|| {
			for entity in entities.iter() {
				world.add_component(*entity, Position::default()).unwrap();
			}
		})
	});
}

fn component_removal(c: &mut Criterion) {
	c.bench_function("removing 1 million components", |b| {
		let mut world = World::new();
		let number_of_entities = 1_000_000;
		let entities = world.create_entities(number_of_entities);
		world.remove_entities(&entities);
		for entity in entities.iter() {
			world.add_component(*entity, Position::default()).unwrap();
		}
		b.iter(|| {
			for entity in entities.iter() {
				world.remove_component::<Position>(*entity).unwrap();
			}
		})
	});
}

fn component_mutation(c: &mut Criterion) {
	c.bench_function("updating 1 million components", |b| {
		let mut world = World::new();
		let number_of_entities = 1_000_000;
		let entities = world.create_entities(number_of_entities);
		for entity in entities.iter() {
			world.add_component(*entity, Position::default()).unwrap();
		}
		b.iter(|| {
			for entity in entities.iter() {
				world.get_component_mut::<Position>(*entity).unwrap().0 = 10.0;
			}
		})
	});
}

fn complex_entities(c: &mut Criterion) {
	c.bench_function("updating 1 million complex entities", |b| {
		let mut world = World::new();
		let number_of_entities = 1_000_000;
		let entities = world.create_entities(number_of_entities);
		for entity in entities.iter() {
			world.add_component(*entity, Position::default()).unwrap();
			world.add_component(*entity, Health::default()).unwrap();
			world
				.add_component(*entity, Name("Test Component".to_string()))
				.unwrap();
		}
		b.iter(|| {
			translation_system(&mut world).expect("Failed to execute translation system!");
		})
	});
}

// Translate only named entities
system!(translation_system, [_resources, _entity], (), (position: Position, name: Name, health: Health) -> anyhow::Result<()> {
	position.0 = 10.0;
	health.0 = 4;
	name.0 = "Renamed".to_string();
	Ok(())
});

fn complex_entity_system(c: &mut Criterion) {
	c.bench_function("updating 1 million complex entities with a system", |b| {
		let mut world = World::new();
		let number_of_entities = 1_000_000;
		let entities = world.create_entities(number_of_entities);
		for entity in entities.iter() {
			world.add_component(*entity, Position::default()).unwrap();
			world.add_component(*entity, Health::default()).unwrap();
			world
				.add_component(*entity, Name("Test Component".to_string()))
				.unwrap();
		}
		b.iter(|| {
			for entity in entities.iter() {
				world.get_component_mut::<Position>(*entity).unwrap().0 = 10.0;
				world.get_component_mut::<Health>(*entity).unwrap().0 = 4;
				world.get_component_mut::<Name>(*entity).unwrap().0 = "Renamed".to_string();
			}
		})
	});
}

criterion_group!(
	name = benches;
	config = Criterion::default().measurement_time(Duration::from_secs(20));
	targets =
		insertion,
		removal,
		reinsertion,
		component_insertion,
		component_removal,
		component_mutation,
		complex_entities,
		complex_entity_system
);

criterion_main!(benches);
