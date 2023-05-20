use hourglass::app::{
	async_trait::async_trait, log, AppEvent, Context, State, StateResult, Transition, WorkerRequest,
};

#[derive(Default)]
pub struct Editor;

#[async_trait]
impl State<Context, AppEvent> for Editor {
	async fn update(
		&mut self,
		_context: &mut Context,
	) -> StateResult<Transition<Context, AppEvent>> {
		log::info!("Hi!");
		Ok(Transition::None)
	}

	async fn on_event(
		&mut self,
		context: &mut Context,
		event: &mut AppEvent,
	) -> StateResult<Transition<Context, AppEvent>> {
		match event {
			AppEvent::Resized { width, height } => {
				log::info!("width: {width} height: {height}");
				Ok(Transition::None)
			}
			AppEvent::Exit => {
				log::info!("Finalizing...");
				context.app_proxy.send_event(WorkerRequest::Exit)?;
				Ok(Transition::None)
			}
		}
	}
}
