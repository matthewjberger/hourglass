use async_channel::{Receiver, Sender};
use std::{
	collections::HashMap,
	error::Error,
	fmt::Debug,
	sync::{
		atomic::{AtomicUsize, Ordering},
		Arc, RwLock,
	},
};

#[derive(Debug, PartialEq)]
pub enum EventBusError {
	ChannelCreationFailed,
	ChannelRemovalFailed,
}

impl std::fmt::Display for EventBusError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			EventBusError::ChannelCreationFailed => write!(f, "Channel creation failed"),
			EventBusError::ChannelRemovalFailed => write!(f, "Channel removal failed"),
		}
	}
}

impl Error for EventBusError {}

type Channel<T> = (Sender<(String, T)>, Receiver<(String, T)>);
type Channels<T> = HashMap<String, Channel<T>>;

pub struct EventBus<T: Clone + Send + 'static> {
	channels: RwLock<Channels<T>>,
}

impl<T: Clone + Send + 'static> Default for EventBus<T> {
	fn default() -> Self {
		Self::new()
	}
}

impl<T: Clone + Send + 'static> EventBus<T> {
	pub fn new() -> Self {
		Self::default()
	}

	pub fn add_channel(&self, channel_name: &str) -> Result<(), EventBusError> {
		let mut channels = self.channels.write().unwrap();
		if channels.contains_key(channel_name) {
			Err(EventBusError::ChannelCreationFailed)
		} else {
			let (sender, receiver) = async_channel::unbounded();
			channels.insert(channel_name.to_string(), (sender, receiver));
			Ok(())
		}
	}

	pub fn remove_channel(&self, channel_name: &str) -> Result<(), EventBusError> {
		let mut channels = self.channels.write().unwrap();
		if channels.contains_key(channel_name) {
			channels.remove(channel_name);
			Ok(())
		} else {
			Err(EventBusError::ChannelRemovalFailed)
		}
	}

	fn get_channel(&self, channel_name: &str) -> Option<Channel<T>> {
		let channels = self.channels.read().unwrap();
		channels.get(channel_name).cloned()
	}
}

pub struct Publisher<T: Clone + Send + 'static> {
	event_bus: Arc<EventBus<T>>,
	channel_name: String,
}

impl<T: Clone + Send + 'static> Publisher<T> {
	pub fn new(event_bus: Arc<EventBus<T>>, channel_name: String) -> Self {
		Publisher {
			event_bus,
			channel_name,
		}
	}

	pub async fn publish(&self, topic: String, payload: T) -> Result<(), EventBusError> {
		if let Some((sender, _)) = self.event_bus.get_channel(&self.channel_name) {
			sender
				.send((topic, payload))
				.await
				.map_err(|_| EventBusError::ChannelRemovalFailed)
		} else {
			Err(EventBusError::ChannelRemovalFailed)
		}
	}
}

#[derive(Debug, PartialEq)]
pub enum SubscriberError {
	SubscriptionFailed(String),
	UnsubscriptionFailed(String),
}

impl std::fmt::Display for SubscriberError {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		match self {
			SubscriberError::SubscriptionFailed(channel_name) => {
				write!(f, "Subscription failed for channel {}", channel_name)
			}
			SubscriberError::UnsubscriptionFailed(channel_name) => {
				write!(f, "Unsubscription failed for channel {}", channel_name)
			}
		}
	}
}

impl Error for SubscriberError {}

pub struct Subscriber<T: Clone + Send + 'static> {
	event_bus: Arc<EventBus<T>>,
	channel_names: Vec<String>,
	current_channel_index: AtomicUsize,
}

impl<T: Clone + Send + 'static> Subscriber<T> {
	pub fn new(event_bus: Arc<EventBus<T>>, channel_names: Vec<String>) -> Self {
		Subscriber {
			event_bus,
			channel_names,
			current_channel_index: AtomicUsize::new(0),
		}
	}

	pub fn subscribe(&self) -> Result<Vec<Receiver<(String, T)>>, EventBusError> {
		self.channel_names
			.iter()
			.map(|channel_name| {
				self.event_bus
					.get_channel(channel_name)
					.map(|(_, receiver)| receiver)
					.ok_or(EventBusError::ChannelRemovalFailed)
			})
			.collect()
	}

	pub async fn try_next_message(&self) -> Option<(String, T)> {
		let index = self.current_channel_index.load(Ordering::Relaxed);
		let channel_name = self.channel_names.get(index)?;
		let (_, receiver) = self.event_bus.get_channel(channel_name)?;
		self.current_channel_index
			.store((index + 1) % self.channel_names.len(), Ordering::Relaxed);
		receiver.try_recv().ok()
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn setup_event_bus() -> Arc<EventBus<String>> {
		let event_bus = Arc::new(EventBus::<String>::new());
		event_bus.add_channel("channel1").unwrap();
		event_bus
	}

	#[async_std::test]
	async fn event_bus_create_channel() {
		let event_bus = Arc::new(EventBus::<String>::new());

		assert_eq!(event_bus.add_channel("channel1"), Ok(()));
		assert_eq!(
			event_bus.add_channel("channel1"),
			Err(EventBusError::ChannelCreationFailed)
		);
	}

	#[async_std::test]
	async fn event_bus_remove_channel() {
		let event_bus = Arc::new(EventBus::<String>::new());

		assert_eq!(event_bus.add_channel("channel1"), Ok(()));
		assert_eq!(event_bus.remove_channel("channel1"), Ok(()));
		assert_eq!(
			event_bus.remove_channel("channel1"),
			Err(EventBusError::ChannelRemovalFailed)
		);
	}

	#[async_std::test]
	async fn publish_and_subscribe() {
		let event_bus = setup_event_bus();

		let _ = event_bus.add_channel("channel1");

		let publisher = Publisher::new(event_bus.clone(), "channel1".to_string());
		assert_eq!(
			publisher
				.publish("topic1".to_string(), "Hello, world!".to_string())
				.await,
			Ok(())
		);

		let subscriber = Subscriber::new(event_bus.clone(), vec!["channel1".to_string()]);
		let receivers = subscriber.subscribe().unwrap();

		let received_messages: Vec<(String, String)> =
			vec![("topic1".to_string(), "Hello, world!".to_string())];
		assert_eq!(receivers[0].recv().await.unwrap(), received_messages[0]);
	}
}
