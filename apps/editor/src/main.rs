use hourglass::app::{
	tokio::{self, sync::mpsc},
	winit::event_loop::EventLoopProxy,
	App, AppConfig, AppMessage, TaskResult, WorkerMessage,
};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
	let app = App::new(&AppConfig::default())?;
	app.run(worker);
	Ok(())
}

async fn worker(
	proxy: EventLoopProxy<AppMessage>,
	mut worker_receiver: mpsc::UnboundedReceiver<WorkerMessage>,
) -> TaskResult {
	loop {
		while let Ok(message) = worker_receiver.try_recv() {
			match message {
				WorkerMessage::Resized { width, height } => {
					println!("Resized: ({width}, {height})");
				}
				WorkerMessage::Exit => {
					println!("Finalizing...");
					proxy.send_event(AppMessage::Exit)?;
				}
			}
		}
		tokio::time::sleep(std::time::Duration::from_millis(500)).await;
	}
}
