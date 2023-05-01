use hourglass::app::{async_trait::async_trait, log, Context, State, StateResult, Transition};

#[derive(Default)]
pub struct Editor;

#[async_trait]
impl State for Editor {
	async fn update(&mut self, _context: &mut Context) -> StateResult<Transition> {
		log::info!("Hi!");
		Ok(Transition::None)
	}
}
