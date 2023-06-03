use std::{
	collections::{HashMap, HashSet, VecDeque},
	fmt,
};

pub type NodeId = usize;

pub struct Node<T> {
	pub id: NodeId,
	pub data: T,
}

#[derive(Debug, Clone, PartialEq)]
pub enum GraphError {
	NodeDoesNotExist(NodeId),
	EdgeAlreadyExists(NodeId, NodeId),
	SelfLoopNotAllowed,
	CycleDetected,
}

impl std::error::Error for GraphError {}

impl std::fmt::Display for GraphError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			GraphError::NodeDoesNotExist(id) => write!(f, "Node with ID {} does not exist", id),
			GraphError::EdgeAlreadyExists(id1, id2) => {
				write!(f, "Edge between nodes {} and {} already exists", id1, id2)
			}
			GraphError::SelfLoopNotAllowed => write!(f, "Self-loops are not allowed"),
			GraphError::CycleDetected => write!(f, "Cycle detected in the graph"),
		}
	}
}

pub struct Graph<T, E> {
	nodes: HashMap<NodeId, Node<T>>,
	adjacency_list: HashMap<NodeId, Vec<(NodeId, E)>>,
}

impl<T, E> Graph<T, E> {
	pub fn new() -> Self {
		Self {
			nodes: HashMap::new(),
			adjacency_list: HashMap::new(),
		}
	}

	pub fn add_node(&mut self, data: T) -> NodeId {
		let node_id = self.nodes.len();
		let node = Node { id: node_id, data };

		self.nodes.insert(node_id, node);
		self.adjacency_list.insert(node_id, Vec::new());

		node_id
	}

	pub fn add_edge(
		&mut self,
		node_id_1: NodeId,
		node_id_2: NodeId,
		edge_weight: E,
	) -> Result<(), GraphError> {
		if node_id_1 == node_id_2 {
			return Err(GraphError::SelfLoopNotAllowed);
		}

		let neighbors1 = self.adjacency_list.get(&node_id_1);
		let neighbors2 = self.adjacency_list.get(&node_id_2);

		match (neighbors1, neighbors2) {
			(Some(_), Some(_)) => {
				if self
					.adjacency_list
					.get(&node_id_1)
					.unwrap()
					.iter()
					.any(|(id, _)| *id == node_id_2)
				{
					return Err(GraphError::EdgeAlreadyExists(node_id_1, node_id_2));
				}

				self.adjacency_list
					.get_mut(&node_id_1)
					.unwrap()
					.push((node_id_2, edge_weight));
				Ok(())
			}
			_ => Err(GraphError::NodeDoesNotExist(if neighbors1.is_none() {
				node_id_1
			} else {
				node_id_2
			})),
		}
	}

	pub fn get_node(&self, node_id: NodeId) -> Option<&Node<T>> {
		self.nodes.get(&node_id)
	}

	pub fn get_node_mut(&mut self, node_id: NodeId) -> Option<&mut Node<T>> {
		self.nodes.get_mut(&node_id)
	}

	pub fn get_edge_weight(&self, node_id_1: NodeId, node_id_2: NodeId) -> Option<&E> {
		self.adjacency_list
			.get(&node_id_1)?
			.iter()
			.find_map(|(id, weight)| if *id == node_id_2 { Some(weight) } else { None })
	}

	pub fn get_edge_weight_mut(&mut self, node_id_1: NodeId, node_id_2: NodeId) -> Option<&mut E> {
		let pos = self
			.adjacency_list
			.get(&node_id_1)?
			.iter()
			.position(|(id, _)| *id == node_id_2)?;
		self.adjacency_list
			.get_mut(&node_id_1)?
			.get_mut(pos)
			.map(|(_, weight)| weight)
	}

	pub fn detect_cycle(&self) -> Result<(), GraphError> {
		let mut visited = HashSet::new();

		for &node_id in self.nodes.keys() {
			if !visited.contains(&node_id) {
				visited.insert(node_id);
				let cycle_detected = self.detect_cycle_recursive(node_id, &mut visited);
				if cycle_detected {
					return Err(GraphError::CycleDetected);
				}
				visited.remove(&node_id);
			}
		}

		Ok(())
	}

	fn detect_cycle_recursive(&self, start_node: NodeId, visited: &mut HashSet<NodeId>) -> bool {
		if let Some(neighbors) = self.adjacency_list.get(&start_node) {
			for &(neighbor, _) in neighbors {
				if !visited.contains(&neighbor) {
					visited.insert(neighbor);
					let cycle_detected = self.detect_cycle_recursive(neighbor, visited);
					if cycle_detected {
						return true;
					}
					visited.remove(&neighbor);
				} else {
					return true;
				}
			}
		}

		false
	}

	pub fn neighbors(&self, id: NodeId) -> Result<&Vec<(NodeId, E)>, GraphError> {
		self.adjacency_list
			.get(&id)
			.ok_or(GraphError::NodeDoesNotExist(id))
	}

	pub fn bfs(&self, start_id: NodeId) -> Result<Vec<NodeId>, GraphError> {
		if !self.nodes.contains_key(&start_id) {
			return Err(GraphError::NodeDoesNotExist(start_id));
		}

		let mut visited = vec![false; self.nodes.len()];
		let mut queue = VecDeque::new();
		let mut order = Vec::new();

		queue.push_back(start_id);
		visited[start_id] = true;

		while let Some(node_id) = queue.pop_front() {
			order.push(node_id);

			if let Some(neighbors) = self.adjacency_list.get(&node_id) {
				for &(neighbor_id, _) in neighbors {
					if !visited[neighbor_id] {
						queue.push_back(neighbor_id);
						visited[neighbor_id] = true;
					}
				}
			}
		}

		Ok(order)
	}

	pub fn dfs(&self, start_id: NodeId) -> Result<Vec<NodeId>, GraphError> {
		if !self.nodes.contains_key(&start_id) {
			return Err(GraphError::NodeDoesNotExist(start_id));
		}

		let mut visited = vec![false; self.nodes.len()];
		let mut stack = Vec::new();
		let mut order = Vec::new();

		stack.push(start_id);

		while let Some(node_id) = stack.pop() {
			if !visited[node_id] {
				visited[node_id] = true;
				order.push(node_id);

				if let Some(neighbors) = self.adjacency_list.get(&node_id) {
					for &(neighbor_id, _) in neighbors {
						if !visited[neighbor_id] {
							stack.push(neighbor_id);
						}
					}
				}
			}
		}

		Ok(order)
	}

	pub fn find_path(
		&self,
		start_id: NodeId,
		end_id: NodeId,
	) -> Result<Option<Vec<NodeId>>, GraphError> {
		if !self.nodes.contains_key(&start_id) {
			return Err(GraphError::NodeDoesNotExist(start_id));
		}
		if !self.nodes.contains_key(&end_id) {
			return Err(GraphError::NodeDoesNotExist(end_id));
		}

		let mut visited = vec![false; self.nodes.len()];
		let mut stack = Vec::new();
		let mut path = Vec::new();

		stack.push(start_id);

		while let Some(node_id) = stack.pop() {
			if !visited[node_id] {
				visited[node_id] = true;
				path.push(node_id);

				if node_id == end_id {
					return Ok(Some(path));
				}

				if let Some(neighbors) = self.adjacency_list.get(&node_id) {
					for &(neighbor_id, _) in neighbors {
						if !visited[neighbor_id] {
							stack.push(neighbor_id);
						}
					}
				}
			}
		}

		Ok(None) // return None if no path exists
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::error::Error;

	fn setup_graph() -> Result<Graph<i32, ()>, Box<dyn Error>> {
		let mut graph = Graph::new();

		let node0 = graph.add_node(0);
		let node1 = graph.add_node(1);
		let node2 = graph.add_node(2);
		let node3 = graph.add_node(3);

		graph.add_edge(node0, node1, ())?;
		graph.add_edge(node0, node2, ())?;
		graph.add_edge(node1, node2, ())?;
		graph.add_edge(node2, node3, ())?;

		Ok(graph)
	}

	#[test]
	fn test_add_node() {
		let mut graph = Graph::<i32, ()>::new();
		assert_eq!(graph.add_node(10), 0);
		assert_eq!(graph.add_node(20), 1);
	}

	#[test]
	fn test_add_edge() {
		let mut graph = Graph::<i32, ()>::new();
		let node0 = graph.add_node(0);
		let node1 = graph.add_node(1);
		assert_eq!(graph.add_edge(node0, node1, ()), Ok(()));
		assert_eq!(
			graph.add_edge(node0, node0, ()),
			Err(GraphError::SelfLoopNotAllowed)
		);
		assert_eq!(
			graph.add_edge(node0, 2, ()),
			Err(GraphError::NodeDoesNotExist(2))
		);
		assert_eq!(
			graph.add_edge(node0, node1, ()),
			Err(GraphError::EdgeAlreadyExists(node0, node1))
		);
	}

	#[test]
	fn test_bfs() -> Result<(), Box<dyn Error>> {
		let graph = setup_graph()?;

		assert_eq!(
			graph.bfs(0).map(|v| v.into_iter().collect::<HashSet<_>>()),
			Ok([0, 1, 2, 3].iter().cloned().collect())
		);
		assert_eq!(
			graph.bfs(1).map(|v| v.into_iter().collect::<HashSet<_>>()),
			Ok([1, 2, 3].iter().cloned().collect())
		);
		assert_eq!(graph.bfs(4), Err(GraphError::NodeDoesNotExist(4)));

		Ok(())
	}

	#[test]
	fn test_dfs() -> Result<(), Box<dyn Error>> {
		let graph = setup_graph()?;
		assert_eq!(
			graph.dfs(0).map(|v| v.into_iter().collect::<HashSet<_>>()),
			Ok([0, 1, 2, 3].iter().cloned().collect())
		);
		assert_eq!(
			graph.dfs(1).map(|v| v.into_iter().collect::<HashSet<_>>()),
			Ok([1, 2, 3].iter().cloned().collect())
		);
		assert_eq!(graph.dfs(4), Err(GraphError::NodeDoesNotExist(4)));
		Ok(())
	}

	#[test]
	fn test_detect_cycle() -> Result<(), Box<dyn Error>> {
		let mut graph = Graph::new();

		let node0 = graph.add_node(0);
		let node1 = graph.add_node(1);
		let node2 = graph.add_node(2);
		let node3 = graph.add_node(3);

		assert_eq!(graph.detect_cycle(), Ok(()));

		graph.add_edge(node0, node1, ())?;
		assert_eq!(graph.detect_cycle(), Ok(()));

		graph.add_edge(node1, node2, ())?;
		assert_eq!(graph.detect_cycle(), Ok(()));

		graph.add_edge(node2, node0, ())?;
		assert_eq!(graph.detect_cycle(), Err(GraphError::CycleDetected));

		graph.add_edge(node2, node3, ())?;
		assert_eq!(graph.detect_cycle(), Err(GraphError::CycleDetected));

		Ok(())
	}

	#[test]
	fn test_find_path() -> Result<(), Box<dyn Error>> {
		let mut graph = Graph::new();

		let node0 = graph.add_node(0);
		let node1 = graph.add_node(1);
		let node2 = graph.add_node(2);
		let node3 = graph.add_node(3);
		let node4 = graph.add_node(4);

		graph.add_edge(node0, node1, ())?;
		graph.add_edge(node0, node2, ())?;
		graph.add_edge(node1, node3, ())?;
		graph.add_edge(node2, node3, ())?;
		graph.add_edge(node3, node4, ())?;

		// Test path from node 0 to 4
		let path = graph.find_path(node0, node4)?;
		assert_eq!(path, Some(vec![0, 2, 3, 4]));

		// Test path from node 1 to 3
		let path = graph.find_path(node1, node3)?;
		assert_eq!(path, Some(vec![1, 3]));

		// Test path where no path exists
		let path = graph.find_path(node4, node0)?;
		assert_eq!(path, None);

		// Test path to nonexistent node
		let path = graph.find_path(node0, 5);
		assert_eq!(path, Err(GraphError::NodeDoesNotExist(5)));

		Ok(())
	}

	#[test]
	fn test_get_node() {
		let mut graph = Graph::<_, ()>::new();
		let node_id = graph.add_node(5);

		// Testing get_node
		match graph.get_node(node_id) {
			Some(node) => assert_eq!(node.data, 5),
			None => panic!("Node does not exist."),
		}

		// Testing get_node_mut
		match graph.get_node_mut(node_id) {
			Some(node) => {
				node.data = 10;
				assert_eq!(node.data, 10);
			}
			None => panic!("Node does not exist."),
		}

		// Confirm that get_node now returns the updated value
		match graph.get_node(node_id) {
			Some(node) => assert_eq!(node.data, 10),
			None => panic!("Node does not exist."),
		}

		// Trying to get a nonexistent node should return None
		assert!(graph.get_node(999).is_none());
	}

	#[test]
	fn test_get_edge_weight() {
		let mut graph = Graph::new();
		let node0 = graph.add_node(0);
		let node1 = graph.add_node(1);
		graph.add_edge(node0, node1, 5).unwrap();

		// Testing get_edge_weight
		match graph.get_edge_weight(node0, node1) {
			Some(weight) => assert_eq!(*weight, 5),
			None => panic!("Edge does not exist."),
		}

		// Testing get_edge_weight_mut
		match graph.get_edge_weight_mut(node0, node1) {
			Some(weight) => {
				*weight = 10;
				assert_eq!(*weight, 10);
			}
			None => panic!("Edge does not exist."),
		}

		// Confirm that get_edge_weight now returns the updated value
		match graph.get_edge_weight(node0, node1) {
			Some(weight) => assert_eq!(*weight, 10),
			None => panic!("Edge does not exist."),
		}

		// Trying to get a nonexistent edge should return None
		assert!(graph.get_edge_weight(node0, 999).is_none());
		assert!(graph.get_edge_weight(999, node1).is_none());
	}
}
