#![forbid(unsafe_code)]

pub mod world;

pub mod error {
	pub type Result<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;
}
