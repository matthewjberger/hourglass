use image::io::Reader;
use std::{future::Future, io};
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

pub type Proxy = EventLoopProxy<AppMessage>;
pub type WorkerReceiver = mpsc::UnboundedReceiver<WorkerMessage>;

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
pub enum AppMessage {
	Exit,
}

#[derive(Debug, Clone)]
pub enum WorkerMessage {
	Resized { width: u32, height: u32 },
	Exit,
}

pub struct App {
	event_loop: EventLoop<AppMessage>,
	window: winit::window::Window,
}

impl App {
	pub fn new(config: &AppConfig) -> Result<Self> {
		let event_loop = EventLoopBuilder::<AppMessage>::with_user_event().build();

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

	pub fn run<T, Fut>(self, task: T)
	where
		T: FnOnce(Proxy, WorkerReceiver) -> Fut + std::marker::Send + 'static,
		Fut: Future<Output = TaskResult> + Send,
	{
		let Self { event_loop, window } = self;

		let (worker_sender, worker_receiver) = mpsc::unbounded_channel();
		let proxy = event_loop.create_proxy();
		task::spawn(async { task(proxy, worker_receiver).await });

		event_loop.run(move |event, _, control_flow| {
			*control_flow = ControlFlow::Poll;

			let process_event = || -> Result<(), Box<dyn std::error::Error>> {
				match event {
					Event::WindowEvent { window_id, event } if window_id == window.id() => {
						match event {
							WindowEvent::CloseRequested => {
								worker_sender.send(WorkerMessage::Exit)?;
							}
							WindowEvent::Resized(PhysicalSize { width, height }) => {
								worker_sender.send(WorkerMessage::Resized { width, height })?
							}
							_ => {}
						}
					}

					// Receive user events from the async task
					Event::UserEvent(message) => match message {
						AppMessage::Exit => {
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
