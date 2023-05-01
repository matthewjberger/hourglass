#![allow(dead_code)]
use gilrs::{Event as GilrsEvent, Gilrs};
use std::path::Path;
use thiserror::Error;
use winit::{
	dpi::PhysicalSize,
	event::{ElementState, KeyboardInput, MouseButton},
};

#[derive(Error, Debug)]
pub enum StateMachineError {
	#[error("Failed to get the current surface texture!")]
	NoStatesPresent,
}

type Result<T, E = StateMachineError> = std::result::Result<T, E>;

pub type StateResult<T, E = Box<dyn std::error::Error>> = std::result::Result<T, E>;

#[derive(Default)]
pub struct Context {
	pub gilrs: Option<Gilrs>,
}

pub struct EmptyState {}
impl State for EmptyState {}

pub trait State: Send + 'static {
	fn label(&self) -> String {
		"Unlabeled Game State".to_string()
	}

	fn on_start(&mut self, _context: &mut Context) -> StateResult<()> {
		Ok(())
	}

	fn on_pause(&mut self, _context: &mut Context) -> StateResult<()> {
		Ok(())
	}

	fn on_stop(&mut self, _context: &mut Context) -> StateResult<()> {
		Ok(())
	}

	fn on_resume(&mut self, _context: &mut Context) -> StateResult<()> {
		Ok(())
	}

	fn update(&mut self, _context: &mut Context) -> StateResult<Transition> {
		Ok(Transition::None)
	}

	fn on_gamepad_event(
		&mut self,
		_context: &mut Context,
		_event: GilrsEvent,
	) -> StateResult<Transition> {
		Ok(Transition::None)
	}

	fn on_file_dropped(&mut self, _context: &mut Context, _path: &Path) -> StateResult<Transition> {
		Ok(Transition::None)
	}

	fn on_resize(
		&mut self,
		_context: &mut Context,
		_physical_size: &PhysicalSize<u32>,
	) -> StateResult<Transition> {
		Ok(Transition::None)
	}

	fn on_mouse(
		&mut self,
		_context: &mut Context,
		_button: &MouseButton,
		_button_state: &ElementState,
	) -> StateResult<Transition> {
		Ok(Transition::None)
	}

	fn on_key(&mut self, _context: &mut Context, _input: KeyboardInput) -> StateResult<Transition> {
		Ok(Transition::None)
	}
}

pub enum Transition {
	None,
	Pop,
	Push(Box<dyn State>),
	Switch(Box<dyn State>),
	Quit,
}

pub struct StateMachine {
	running: bool,
	states: Vec<Box<dyn State>>,
}

impl StateMachine {
	pub fn new(initial_state: impl State + 'static) -> Self {
		Self {
			running: false,
			states: vec![Box::new(initial_state)],
		}
	}

	pub fn active_state_label(&self) -> Option<String> {
		if !self.running {
			return None;
		}
		self.states.last().map(|state| state.label())
	}

	pub fn is_running(&self) -> bool {
		self.running
	}

	pub fn start(&mut self, context: &mut Context) -> StateResult<()> {
		if self.running {
			return Ok(());
		}
		self.running = true;
		self.active_state_mut()?.on_start(context)
	}

	pub fn update(&mut self, context: &mut Context) -> StateResult<()> {
		if !self.running {
			return Ok(());
		}
		let transition = self.active_state_mut()?.update(context)?;
		self.transition(transition, context)
	}

	pub fn on_gamepad_event(
		&mut self,
		context: &mut Context,
		event: GilrsEvent,
	) -> StateResult<()> {
		if !self.running {
			return Ok(());
		}
		let transition = self.active_state_mut()?.on_gamepad_event(context, event)?;
		self.transition(transition, context)
	}

	pub fn on_file_dropped(&mut self, context: &mut Context, path: &Path) -> StateResult<()> {
		if !self.running {
			return Ok(());
		}
		let transition = self.active_state_mut()?.on_file_dropped(context, path)?;
		self.transition(transition, context)
	}

	pub fn on_resize(
		&mut self,
		context: &mut Context,
		physical_size: &PhysicalSize<u32>,
	) -> StateResult<()> {
		if !self.running {
			return Ok(());
		}
		let transition = self.active_state_mut()?.on_resize(context, physical_size)?;
		self.transition(transition, context)
	}

	pub fn on_mouse(
		&mut self,
		context: &mut Context,
		button: &MouseButton,
		button_state: &ElementState,
	) -> StateResult<()> {
		if !self.running {
			return Ok(());
		}
		let transition = self
			.active_state_mut()?
			.on_mouse(context, button, button_state)?;
		self.transition(transition, context)
	}

	pub fn on_key(&mut self, context: &mut Context, input: KeyboardInput) -> StateResult<()> {
		if !self.running {
			return Ok(());
		}
		let transition = self.active_state_mut()?.on_key(context, input)?;
		self.transition(transition, context)
	}

	fn transition(&mut self, request: Transition, context: &mut Context) -> StateResult<()> {
		if !self.running {
			return Ok(());
		}
		match request {
			Transition::None => Ok(()),
			Transition::Pop => self.pop(context),
			Transition::Push(state) => self.push(state, context),
			Transition::Switch(state) => self.switch(state, context),
			Transition::Quit => self.stop(context),
		}
	}

	fn active_state_mut(&mut self) -> Result<&mut Box<(dyn State + 'static)>> {
		self.states
			.last_mut()
			.ok_or(StateMachineError::NoStatesPresent)
	}

	fn switch(&mut self, state: Box<dyn State>, context: &mut Context) -> StateResult<()> {
		if !self.running {
			return Ok(());
		}
		if let Some(mut state) = self.states.pop() {
			state.on_stop(context)?;
		}
		self.states.push(state);
		self.active_state_mut()?.on_start(context)
	}

	fn push(&mut self, state: Box<dyn State>, context: &mut Context) -> StateResult<()> {
		if !self.running {
			return Ok(());
		}
		if let Ok(state) = self.active_state_mut() {
			state.on_pause(context)?;
		}
		self.states.push(state);
		self.active_state_mut()?.on_start(context)
	}

	fn pop(&mut self, context: &mut Context) -> StateResult<()> {
		if !self.running {
			return Ok(());
		}

		if let Some(mut state) = self.states.pop() {
			state.on_stop(context)?;
		}

		if let Some(state) = self.states.last_mut() {
			state.on_resume(context)
		} else {
			self.running = false;
			Ok(())
		}
	}

	pub fn stop(&mut self, context: &mut Context) -> StateResult<()> {
		if !self.running {
			return Ok(());
		}
		while let Some(mut state) = self.states.pop() {
			state.on_stop(context)?;
		}
		self.running = false;
		Ok(())
	}
}
