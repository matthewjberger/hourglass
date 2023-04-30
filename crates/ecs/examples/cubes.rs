use anyhow::Result;
use ecs::{system, world::World};
use kiss3d::{camera::ArcBall, light::Light, scene::SceneNode, window::Window};
use nalgebra::{Point3, UnitQuaternion, Vector3};
use rand::Rng;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

fn main() -> Result<()> {
	let mut window = Window::new("Entity-Component-System Architecture Demo");
	window.set_light(Light::StickToCamera);

	let mut world = create_world(&mut window);

	let mut arc_ball = create_camera();

	let color_system = ColorSystem::new();
	while window.render_with_camera(&mut arc_ball) {
		rotation_system(0.014, &mut world)?;
		color_system.run(&mut world)?;
	}

	Ok(())
}

fn create_world(window: &mut Window) -> World {
	let mut rng = rand::thread_rng();
	let mut world = World::new();
	let entities = world.create_entities(10);
	for entity in entities {
		let mut node = window.add_cube(1.0, 1.0, 1.0);
		node.set_color(0.0, 1.0, 0.0);
		node.set_visible(true);
		node.set_local_translation(
			[
				rng.gen_range(-5.0..5.0),
				rng.gen_range(-5.0..5.0),
				rng.gen_range(-5.0..5.0),
			]
			.into(),
		);
		world.add_component(entity, node).unwrap();
	}
	world
}

system!(rotation_system, [_resources, _entity], (value: f32), (node: SceneNode) -> Result<()> {
	node.prepend_to_local_rotation(&UnitQuaternion::from_axis_angle(&Vector3::y_axis(), value));
	Ok(())
});

struct ColorSystem {
	start_time: Duration,
}

impl ColorSystem {
	pub fn new() -> Self {
		Self {
			start_time: SystemTime::now().duration_since(UNIX_EPOCH).unwrap(),
		}
	}

	system!(run, [_resources, _entity], (self: &Self), (node: SceneNode) -> Result<()> {
		let time = (SystemTime::now().duration_since(UNIX_EPOCH).unwrap() - self.start_time).as_secs_f32();
		node.set_color(time.sin(), time.cos(), 0.5);
		Ok(())
	});
}

fn create_camera() -> ArcBall {
	let eye = Point3::new(10.0, 10.0, 10.0);
	let at = Point3::origin();
	ArcBall::new(eye, at)
}
