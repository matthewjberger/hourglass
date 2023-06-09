use crate::state::{State, StateMachine};
use image::io::Reader;
use std::io;
use thiserror::Error;
use tokio::{sync::mpsc, task};
use winit::{
	self,
	dpi::PhysicalSize,
	error::OsError,
	event::{Event, WindowEvent},
	event_loop::{ControlFlow, EventLoop, EventLoopBuilder, EventLoopProxy},
	window::{Icon, WindowBuilder},
};

#[derive(Error, Debug)]
pub enum Error {
	#[error("Failed to create icon file!")]
	CreateIcon(#[source] winit::window::BadIcon),

	#[error("Failed to create a window!")]
	CreateWindow(#[source] OsError),

	#[error("Failed to decode icon file at path: {1}")]
	DecodeIconFile(#[source] image::ImageError, String),

	#[error("Failed to open icon file at path: {1}")]
	OpenIconFile(#[source] io::Error, String),
}

type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub struct AppConfig {
	pub width: u32,
	pub height: u32,
	pub is_fullscreen: bool,
	pub title: String,
	pub icon: Option<String>,
}

impl Default for AppConfig {
	fn default() -> Self {
		Self {
			width: 1920,
			height: 1080,
			is_fullscreen: false,
			title: "Hourglass App".to_string(),
			icon: None,
		}
	}
}

pub type TaskResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(Debug, Clone)]
pub enum WorkerRequest {
	Exit,
}

#[derive(Debug, Clone)]
pub enum AppEvent {
	Resized { width: u32, height: u32 },
	Exit,
}

pub struct Context {
	pub app_proxy: EventLoopProxy<WorkerRequest>,
}

pub struct App {
	event_loop: EventLoop<WorkerRequest>,
	window: winit::window::Window,
}

impl App {
	pub fn new(config: &AppConfig) -> Result<Self> {
		let event_loop = EventLoopBuilder::<WorkerRequest>::with_user_event().build();

		let mut window_builder = WindowBuilder::new()
			.with_title(config.title.to_string())
			.with_inner_size(PhysicalSize::new(config.width, config.height));

		if let Some(icon_path) = config.icon.as_ref() {
			let icon = load_icon(icon_path)?;
			window_builder = window_builder.with_window_icon(Some(icon));
		}

		let window = window_builder
			.build(&event_loop)
			.map_err(Error::CreateWindow)?;

		Ok(Self { window, event_loop })
	}

	pub fn run(self, initial_state: impl State<Context, AppEvent>) {
		let Self { event_loop, window } = self;

		let (worker_sender, worker_receiver) = mpsc::unbounded_channel();
		let proxy = event_loop.create_proxy();
		task::spawn(worker(proxy, worker_receiver, initial_state));

		event_loop.run(move |event, _, control_flow| {
			*control_flow = ControlFlow::Poll;

			let process_event = || -> Result<(), Box<dyn std::error::Error>> {
				match event {
					// Respond to winit events by notifying the background worker
					Event::WindowEvent { window_id, event } if window_id == window.id() => {
						match event {
							WindowEvent::CloseRequested => {
								worker_sender.send(AppEvent::Exit)?;
							}
							WindowEvent::Resized(PhysicalSize { width, height }) => {
								worker_sender.send(AppEvent::Resized { width, height })?
							}
							_ => {}
						}
					}

					// These events are sent the background worker
					Event::UserEvent(message) => match message {
						WorkerRequest::Exit => {
							*control_flow = ControlFlow::Exit;
						}
					},
					_ => {}
				}

				Ok(())
			};

			if let Err(error) = process_event() {
				log::error!("Error: {error}");
			}
		});
	}
}

fn load_icon(icon_path: &String) -> Result<Icon, Error> {
	let image = Reader::open(icon_path)
		.map_err(|error| Error::OpenIconFile(error, icon_path.to_string()))?
		.decode()
		.map_err(|error| Error::DecodeIconFile(error, icon_path.to_string()))?
		.into_rgba8();
	let (width, height) = image.dimensions();
	let icon = Icon::from_rgba(image.into_raw(), width, height).map_err(Error::CreateIcon)?;
	Ok(icon)
}

async fn worker(
	app_proxy: EventLoopProxy<WorkerRequest>,
	mut worker_receiver: mpsc::UnboundedReceiver<AppEvent>,
	initial_state: impl State<Context, AppEvent>,
) -> TaskResult {
	let mut state_machine = StateMachine::new(initial_state);

	let mut context = Context { app_proxy };
	state_machine.start(&mut context).await?;

	loop {
		while let Ok(mut event) = worker_receiver.try_recv() {
			state_machine.on_event(&mut context, &mut event).await?;
		}

		if let Err(error) = state_machine.update(&mut context).await {
			log::warn!("{error}");
		}

		tokio::time::sleep(std::time::Duration::from_millis(500)).await;
	}
}
