use std::{
	any::{Any, TypeId},
	collections::HashMap,
};

#[derive(Default)]
pub struct ResourceMap {
	data: HashMap<TypeId, Box<dyn Any + 'static>>,
}

impl ResourceMap {
	pub fn new() -> Self {
		Self::default()
	}
}

impl ResourceMap {
	/// Retrieve the value stored in the map for the type `T`, if it exists.
	pub fn get<T: 'static>(&self) -> Option<&T> {
		self.data
			.get(&TypeId::of::<T>())
			.and_then(|any| any.downcast_ref())
	}

	/// Retrieve a mutable reference to the value stored in the map for the type `T`, if it exists.
	pub fn get_mut<T: 'static>(&mut self) -> Option<&mut T> {
		self.data
			.get_mut(&TypeId::of::<T>())
			.and_then(|any| any.downcast_mut())
	}

	/// Set the value contained in the map for the type `T`.
	/// This will override any previous value stored.
	pub fn insert<T: 'static>(&mut self, value: T) {
		self.data.insert(TypeId::of::<T>(), Box::new(value) as _);
	}

	/// Remove the value for the type `T` if it existed.
	pub fn remove<T: 'static>(&mut self) {
		self.data.remove(&TypeId::of::<T>());
	}
}

#[cfg(test)]
mod tests {
	use super::ResourceMap;

	#[derive(Debug, Default, PartialEq, Eq, Copy, Clone)]
	pub struct Viewport {
		width: u32,
		height: u32,
	}

	#[test]
	fn resources() {
		let mut resources = ResourceMap::new();

		resources.insert(Viewport::default());
		assert_eq!(resources.get::<Viewport>(), Some(&Viewport::default()));

		let (width, height) = (1920, 1080);
		let mut viewport = resources.get_mut::<Viewport>().unwrap();
		viewport.width = width;
		viewport.height = height;
		assert_eq!(
			resources.get::<Viewport>(),
			Some(&Viewport { width, height })
		);

		resources.remove::<Viewport>();
		assert_eq!(resources.get::<Viewport>(), None);
	}
}
