use self::error::GenerationError;
use std::ops::{Deref, DerefMut};

pub mod error {
	use super::*;

	#[derive(Debug)]
	pub struct GenerationError {
		pub handle: Handle,
	}

	impl std::error::Error for GenerationError {}

	impl std::fmt::Display for GenerationError {
		fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
			write!(f, "Entity '{:?}' generation i.", self.handle)
		}
	}

	#[derive(Debug)]
	pub struct HandleNotFoundError {
		pub handle: Handle,
	}

	impl std::error::Error for HandleNotFoundError {}

	impl std::fmt::Display for HandleNotFoundError {
		fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
			write!(f, "Entity '{:?}' does not exist.", self.handle)
		}
	}
}

pub type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

pub type SlotVec<T> = Vec<Option<Slot<T>>>;

#[derive(Default, Debug, PartialEq, Eq, Copy, Clone, Hash)]
pub struct Handle {
	index: usize,
	generation: usize,
}

impl Handle {
	pub const fn index(&self) -> &usize {
		&self.index
	}

	pub const fn generation(&self) -> &usize {
		&self.generation
	}
}

pub struct GenerationalVec<T> {
	elements: SlotVec<T>,
}

impl<T> GenerationalVec<T> {
	pub fn new(elements: SlotVec<T>) -> Self {
		Self { elements }
	}

	pub fn insert(&mut self, handle: Handle, value: T) -> Result<()> {
		while self.elements.len() <= handle.index {
			self.elements.push(None);
		}

		let previous_generation = match self.elements.get(handle.index) {
			Some(Some(entry)) => entry.generation,
			_ => 0,
		};

		if previous_generation > handle.generation {
			return Err(Box::new(GenerationError { handle }));
		}

		self.elements[handle.index] = Some(Slot {
			value,
			generation: handle.generation,
		});

		Ok(())
	}

	pub fn remove(&mut self, handle: Handle) {
		if let Some(e) = self.elements.get_mut(handle.index) {
			*e = None;
		}
	}

	pub fn get(&self, handle: Handle) -> Option<&T> {
		if handle.index >= self.elements.len() {
			return None;
		}
		self.elements[handle.index]
			.as_ref()
			.filter(|c| c.generation == handle.generation)
			.map(|entry| &entry.value)
	}

	pub fn get_mut(&mut self, handle: Handle) -> Option<&mut T> {
		if handle.index >= self.elements.len() {
			return None;
		}
		self.elements[handle.index]
			.as_mut()
			.filter(|c| c.generation == handle.generation)
			.map(|entry| &mut entry.value)
	}
}

impl<T> Deref for GenerationalVec<T> {
	type Target = SlotVec<T>;

	fn deref(&self) -> &Self::Target {
		&self.elements
	}
}

impl<T> DerefMut for GenerationalVec<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.elements
	}
}

pub struct Slot<T> {
	value: T,
	generation: usize,
}

impl<T> Slot<T> {
	pub const fn new(value: T, generation: usize) -> Self {
		Self { value, generation }
	}

	pub const fn generation(&self) -> &usize {
		&self.generation
	}
}

impl<T> Deref for Slot<T> {
	type Target = T;

	fn deref(&self) -> &Self::Target {
		&self.value
	}
}

impl<T> DerefMut for Slot<T> {
	fn deref_mut(&mut self) -> &mut Self::Target {
		&mut self.value
	}
}

pub struct Allocation {
	allocated: bool,
	generation: usize,
}

#[derive(Default)]
pub struct HandleAllocator {
	allocations: Vec<Allocation>,
	available_handles: Vec<usize>,
}

impl HandleAllocator {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn allocate(&mut self) -> Handle {
		match self.available_handles.pop() {
			Some(index) => {
				self.allocations[index].generation += 1;
				self.allocations[index].allocated = true;
				Handle {
					index,
					generation: self.allocations[index].generation,
				}
			}
			None => {
				self.allocations.push(Allocation {
					allocated: true,
					generation: 0,
				});
				Handle {
					index: self.allocations.len() - 1,
					generation: 0,
				}
			}
		}
	}

	pub fn deallocate(&mut self, handle: &Handle) {
		if !self.is_allocated(handle) {
			return;
		}
		self.allocations[handle.index].allocated = false;
		self.available_handles.push(handle.index);
	}

	pub fn is_allocated(&self, handle: &Handle) -> bool {
		self.handle_exists(handle)
			&& self.allocations[handle.index].generation == handle.generation
			&& self.allocations[handle.index].allocated
	}

	pub fn handle_exists(&self, handle: &Handle) -> bool {
		handle.index < self.allocations.len()
	}

	pub fn allocated_handles(&self) -> Vec<Handle> {
		self.allocations
			.iter()
			.enumerate()
			.filter(|(_, allocation)| allocation.allocated)
			.map(|(index, allocation)| Handle {
				index,
				generation: allocation.generation,
			})
			.collect()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn insertion_and_removal() -> Result<()> {
		let mut elements = GenerationalVec::new(SlotVec::<u32>::default());
		let mut handle_allocator = HandleAllocator::new();

		// allocate a handle
		let handle = handle_allocator.allocate();
		elements.insert(handle, 3)?;
		assert_eq!(elements.get(handle), Some(&3));

		// modify an existing handle
		if let Some(element) = elements.get_mut(handle) {
			*element = 10;
		}
		assert_eq!(elements.get(handle), Some(&10));

		// Clear a handle's slot
		elements.remove(handle);
		assert_eq!(elements.get(handle), None);

		// Deallocate a handle
		handle_allocator.deallocate(&handle);
		assert!(!handle_allocator.is_allocated(&handle));

		// This assures that the "A->B->A" problem is addressed
		let next_handle = handle_allocator.allocate();
		assert_eq!(
			next_handle,
			Handle {
				index: handle.index,
				generation: handle.index + 1,
			}
		);

		Ok(())
	}

	#[test]
	fn allocated_handles() -> Result<()> {
		let mut handle_allocator = HandleAllocator::new();

		let first_handle = handle_allocator.allocate();
		assert!(handle_allocator.is_allocated(&first_handle));
		assert_eq!(handle_allocator.allocated_handles(), &[first_handle]);

		let second_handle = handle_allocator.allocate();
		assert!(handle_allocator.is_allocated(&second_handle));
		assert_eq!(
			handle_allocator.allocated_handles(),
			&[first_handle, second_handle]
		);

		Ok(())
	}

	#[test]
	fn test_insert() {
		let mut vec = GenerationalVec::new(Vec::new());

		let handle = HandleAllocator::new().allocate();
		assert!(vec.insert(handle, 10).is_ok());

		assert_eq!(*vec.get(handle).unwrap(), 10);
	}

	#[test]
	fn test_remove() {
		let mut vec = GenerationalVec::new(Vec::new());

		let handle = HandleAllocator::new().allocate();
		vec.insert(handle, 10).unwrap();

		vec.remove(handle);

		assert!(vec.get(handle).is_none());
	}

	#[test]
	fn test_get_mut() {
		let mut vec = GenerationalVec::new(Vec::new());

		let handle = HandleAllocator::new().allocate();
		vec.insert(handle, 10).unwrap();

		*vec.get_mut(handle).unwrap() = 20;

		assert_eq!(*vec.get(handle).unwrap(), 20);
	}

	#[test]
	fn test_invalid_generation() {
		let mut vec = GenerationalVec::new(Vec::new());

		let handle = HandleAllocator::new().allocate();
		vec.insert(handle, 10).unwrap();

		// Modify the handle to have an invalid generation
		let invalid_handle = Handle {
			generation: handle.generation() + 1,
			..handle
		};

		assert!(vec.get(invalid_handle).is_none());
		assert!(vec.get_mut(invalid_handle).is_none());
	}

	#[test]
	fn test_generational_vec() -> Result<(), Box<dyn std::error::Error>> {
		let mut allocator = HandleAllocator::new();
		let handle1 = allocator.allocate();
		let handle2 = allocator.allocate();
		let handle3 = allocator.allocate();

		let mut vec = GenerationalVec::new(Vec::new());

		assert!(vec.get(handle1).is_none());
		assert!(vec.get(handle2).is_none());
		assert!(vec.get(handle3).is_none());

		vec.insert(handle1, "value1".to_string())?;
		vec.insert(handle2, "value2".to_string())?;
		vec.insert(handle3, "value3".to_string())?;

		assert_eq!(vec.get(handle1), Some(&"value1".to_string()));
		assert_eq!(vec.get(handle2), Some(&"value2".to_string()));
		assert_eq!(vec.get(handle3), Some(&"value3".to_string()));

		vec.remove(handle1);
		assert!(vec.get(handle1).is_none());
		assert_eq!(vec.get(handle2), Some(&"value2".to_string()));
		assert_eq!(vec.get(handle3), Some(&"value3".to_string()));

		allocator.deallocate(&handle1);
		allocator.deallocate(&handle2);
		allocator.deallocate(&handle3);

		assert!(!allocator.is_allocated(&handle1));
		assert!(!allocator.is_allocated(&handle2));
		assert!(!allocator.is_allocated(&handle3));

		Ok(())
	}
}
