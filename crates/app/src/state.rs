#![allow(dead_code)]

use async_trait::async_trait;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StateMachineError {
	#[error("No states present in state machine.")]
	NoStatesPresent,
}

type Result<T, E = StateMachineError> = std::result::Result<T, E>;
pub type StateResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[async_trait]
pub trait State<T, E>: Send + 'static {
	fn label(&self) -> String {
		"Unlabeled State".to_string()
	}

	// This state has been pushed onto the state stack
	async fn on_start(&mut self, _context: &mut T) -> StateResult<()> {
		Ok(())
	}

	// This state has been pushed onto an existing state
	async fn on_suspend(&mut self, _context: &mut T) -> StateResult<()> {
		Ok(())
	}

	// This state has been popped off the state stack
	async fn on_stop(&mut self, _context: &mut T) -> StateResult<()> {
		Ok(())
	}

	// A state stacked on top of the this state has been popped off the state stack
	async fn on_resume(&mut self, _context: &mut T) -> StateResult<()> {
		Ok(())
	}

	// Main function for states, called every loop
	async fn update(&mut self, _context: &mut T) -> StateResult<Transition<T, E>> {
		Ok(Transition::None)
	}

	// Pass an event structure into the current state
	// for updates that can't occur every loop
	async fn on_event(
		&mut self,
		_context: &mut T,
		_event: &mut E,
	) -> StateResult<Transition<T, E>> {
		Ok(Transition::None)
	}
}

pub enum Transition<T, E> {
	None,
	Pop,
	Push(Box<dyn State<T, E>>),
	Switch(Box<dyn State<T, E>>),
	Quit,
}

pub struct StateMachine<T, E> {
	running: bool,
	states: Vec<Box<dyn State<T, E>>>,
}

impl<T: 'static, E: 'static> StateMachine<T, E> {
	pub fn new(initial_state: impl State<T, E> + 'static) -> Self {
		Self {
			running: false,
			states: vec![Box::new(initial_state)],
		}
	}

	pub async fn active_state_label(&self) -> Option<String> {
		if !self.running {
			return None;
		}
		self.states.last().map(|state| state.label())
	}

	pub async fn is_running(&self) -> bool {
		self.running
	}

	pub async fn start(&mut self, context: &mut T) -> StateResult<()> {
		if self.running {
			return Ok(());
		}
		self.running = true;
		self.active_state_mut()?.on_start(context).await
	}

	pub async fn on_event(&mut self, context: &mut T, event: &mut E) -> StateResult<()> {
		if !self.running {
			return Ok(());
		}
		let transition = self.active_state_mut()?.on_event(context, event).await?;
		self.transition(transition, context).await
	}

	pub async fn update(&mut self, context: &mut T) -> StateResult<()> {
		if !self.running {
			return Ok(());
		}
		let transition = self.active_state_mut()?.update(context).await?;
		self.transition(transition, context).await
	}

	async fn transition(&mut self, request: Transition<T, E>, context: &mut T) -> StateResult<()> {
		if !self.running {
			return Ok(());
		}
		match request {
			Transition::None => Ok(()),
			Transition::Pop => self.pop(context).await,
			Transition::Push(state) => self.push(state, context).await,
			Transition::Switch(state) => self.switch(state, context).await,
			Transition::Quit => self.stop(context).await,
		}
	}

	fn active_state_mut(&mut self) -> Result<&mut Box<(dyn State<T, E> + 'static)>> {
		self.states
			.last_mut()
			.ok_or(StateMachineError::NoStatesPresent)
	}

	async fn switch(&mut self, state: Box<dyn State<T, E>>, context: &mut T) -> StateResult<()> {
		if !self.running {
			return Ok(());
		}
		if let Some(mut state) = self.states.pop() {
			state.on_stop(context).await?;
		}
		self.states.push(state);
		self.active_state_mut()?.on_start(context).await
	}

	async fn push(&mut self, state: Box<dyn State<T, E>>, context: &mut T) -> StateResult<()> {
		if !self.running {
			return Ok(());
		}
		if let Ok(state) = self.active_state_mut() {
			state.on_suspend(context).await?;
		}
		self.states.push(state);
		self.active_state_mut()?.on_start(context).await
	}

	async fn pop(&mut self, context: &mut T) -> StateResult<()> {
		if !self.running {
			return Ok(());
		}

		if let Some(mut state) = self.states.pop() {
			state.on_stop(context).await?;
		}

		if let Some(state) = self.states.last_mut() {
			state.on_resume(context).await
		} else {
			self.running = false;
			Ok(())
		}
	}

	pub async fn stop(&mut self, context: &mut T) -> StateResult<()> {
		if !self.running {
			return Ok(());
		}
		while let Some(mut state) = self.states.pop() {
			state.on_stop(context).await?;
		}
		self.running = false;
		Ok(())
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::sync::Arc;
	use tokio::sync::Mutex;

	struct MockState {
		label: String,
		counter: Arc<Mutex<u32>>,
	}

	impl MockState {
		fn new(label: &str, counter: Arc<Mutex<u32>>) -> Self {
			MockState {
				label: label.to_string(),
				counter,
			}
		}
	}

	#[async_trait]
	impl State<(), ()> for MockState {
		fn label(&self) -> String {
			self.label.clone()
		}

		async fn on_start(&mut self, _context: &mut ()) -> StateResult<()> {
			let mut counter = self.counter.lock().await;
			*counter += 1;
			Ok(())
		}
	}

	#[tokio::test]
	async fn test_initial_state() {
		let counter = Arc::new(Mutex::new(0));
		let state = MockState::new("TestState", counter.clone());
		let state_machine = StateMachine::new(state);

		assert_eq!(state_machine.is_running().await, false);
		assert_eq!(state_machine.active_state_label().await, None);
	}

	#[tokio::test]
	async fn test_start_state_machine() {
		let counter = Arc::new(Mutex::new(0));
		let state = MockState::new("TestState", counter.clone());
		let mut state_machine = StateMachine::new(state);
		let mut context = ();

		state_machine.start(&mut context).await.unwrap();

		assert_eq!(state_machine.is_running().await, true);
		assert_eq!(
			state_machine.active_state_label().await,
			Some("TestState".to_string())
		);
		assert_eq!(*counter.lock().await, 1);
	}

	#[tokio::test]
	async fn test_state_transition() {
		let counter = Arc::new(Mutex::new(0));
		let state = MockState::new("TestState", counter.clone());
		let mut state_machine = StateMachine::new(state);
		let mut context = ();

		state_machine.start(&mut context).await.unwrap();

		let state2 = MockState::new("TestState2", counter.clone());
		state_machine
			.transition(Transition::Push(Box::new(state2)), &mut context)
			.await
			.unwrap();

		assert_eq!(
			state_machine.active_state_label().await,
			Some("TestState2".to_string())
		);

		state_machine
			.transition(Transition::Pop, &mut context)
			.await
			.unwrap();

		assert_eq!(
			state_machine.active_state_label().await,
			Some("TestState".to_string())
		);
	}

	#[tokio::test]
	async fn test_stop_state_machine() {
		let counter = Arc::new(Mutex::new(0));
		let state = MockState::new("TestState", counter.clone());
		let mut state_machine = StateMachine::new(state);
		let mut context = ();

		state_machine.start(&mut context).await.unwrap();
		state_machine.stop(&mut context).await.unwrap();

		assert_eq!(state_machine.is_running().await, false);
		assert_eq!(state_machine.active_state_label().await, None);
	}
}
