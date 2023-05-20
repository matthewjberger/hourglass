#![forbid(unsafe_code)]

mod editor;

use editor::Editor;
use hourglass::app::{tokio, App, AppConfig};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
	env_logger::init();
	let app = App::new(&AppConfig::default())?;
	app.run(Editor);
	Ok(())
}
