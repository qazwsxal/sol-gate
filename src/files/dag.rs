use std::{
    collections::{HashMap, HashSet, VecDeque},
    hash::Hash,
};

/// This is a very basic implemtation of a DAG
/// Can't remove nodes, doesn't enforce DAGness
/// Plz do not abuse.
#[derive(Debug)]
pub struct HashDAG<T: Hash + Eq + Clone, U: Clone> {
    //n.b. we don't actually enforce that this is a DAG, things might break if misused!
    nodes: Vec<HashNode<T>>,
    node_map: HashMap<T, usize>,
    edge_map: HashMap<(usize, usize), U>,
}
#[derive(Debug, thiserror::Error)]
pub enum DAGError {
    #[error("Node not found")]
    NoNode,
}

#[derive(Debug)]
pub struct HashNode<T> {
    id: T,
    parents: HashSet<usize>,
    children: HashSet<usize>,
}

impl<T> HashNode<T> {
    pub fn new(id: T) -> HashNode<T> {
        HashNode {
            id,
            parents: HashSet::<usize>::new(),
            children: HashSet::<usize>::new(),
        }
    }

    pub fn data(&self) -> &T {
        &self.id
    }
}

impl<T: Hash + Eq + Clone, U: Clone> HashDAG<T, U> {
    pub fn new() -> HashDAG<T, U> {
        HashDAG {
            nodes: Vec::<HashNode<T>>::new(),
            node_map: HashMap::<T, usize>::new(),
            edge_map: HashMap::<(usize, usize), U>::new(),
        }
    }
    pub fn get_nodes(&self) -> &Vec<HashNode<T>> {
        &self.nodes
    }

    pub fn add(&mut self, id: &T) {
        if self.node_map.contains_key(id) {
            return;
        }
        let node = HashNode::new(id.clone());
        let index = self.nodes.len(); //we're always appending, so the new index is the length of the node vec
        self.nodes.push(node);
        self.node_map.insert(id.clone(), index);
    }

    pub fn add_relationship(
        &mut self,
        child: &T,
        parent: &T,
        edge_data: U,
    ) -> Result<(), DAGError> {
        if !self.node_map.contains_key(child) {
            return Err(DAGError::NoNode);
        }
        if !self.node_map.contains_key(parent) {
            // make sure the parent exists.
            return Err(DAGError::NoNode);
        }
        let par_idx = self.node_map.get(parent).unwrap();
        let chi_idx = self.node_map.get(child).unwrap();
        {
            let par_node = self.nodes.get_mut(*par_idx).unwrap();
            par_node.children.insert(chi_idx.clone());
        }
        {
            let chi_node = self.nodes.get_mut(*chi_idx).unwrap();
            chi_node.parents.insert(par_idx.clone());
        }

        self.edge_map.insert((*par_idx, *chi_idx), edge_data);
        Ok(())
    }

    pub fn get_edge_data(&self, child: &T, parent: &T) -> Option<&U> {
        let chi_opt = self.node_map.get(child);
        let par_opt = self.node_map.get(parent);
        if let (Some(chi_idx), Some(par_idx)) = (chi_opt, par_opt) {
            self.edge_map.get(&(*chi_idx, *par_idx))
        } else {
            None
        }
    }

    pub fn add_children(
        &mut self,
        children: &Vec<T>,
        parent: &T,
        edge_data: &Vec<U>,
    ) -> Result<(), DAGError> {
        for child in children {
            if !self.node_map.contains_key(child) {
                return Err(DAGError::NoNode);
            }
        }
        if !self.node_map.contains_key(parent) {
            return Err(DAGError::NoNode);
        }
        let chi_idxs = children
            .iter()
            .map(|child| self.node_map.get(child).unwrap())
            .cloned()
            .collect::<Vec<_>>();
        let par_idx = self.node_map.get(parent).unwrap();
        {
            let par_node = self.nodes.get_mut(*par_idx).unwrap();
            par_node.children.extend(chi_idxs.iter().cloned());
        }
        for (chi_idx, ed) in chi_idxs.iter().zip(edge_data) {
            let chi_node = self.nodes.get_mut(*chi_idx).unwrap();
            chi_node.parents.insert(par_idx.clone());
            self.edge_map.insert((*par_idx, *chi_idx), ed.clone());
        }
        Ok(())
    }

    fn children_by_idx(&self, idx: &usize) -> HashSet<usize> {
        self.nodes.get(*idx).unwrap().children.clone()
    }

    fn parents_by_idx(&self, idx: &usize) -> HashSet<usize> {
        self.nodes.get(*idx).unwrap().parents.clone()
    }

    pub fn children(&self, id: &T) -> Option<HashSet<T>> {
        if let Some(par_index) = self.node_map.get(&id) {
            Some(
                self.children_by_idx(par_index)
                    .iter()
                    .map(|idx| self.nodes.get(*idx).unwrap().id.clone())
                    .collect::<HashSet<T>>(),
            )
        } else {
            None
        }
    }

    pub fn parents(&self, id: &T) -> Option<HashSet<T>> {
        if let Some(child_index) = self.node_map.get(&id) {
            Some(
                self.parents_by_idx(child_index)
                    .iter()
                    .map(|&idx| self.nodes.get(idx).unwrap().id.clone())
                    .collect::<HashSet<T>>(),
            )
        } else {
            None
        }
    }

    pub fn descendants(&self, id: &T) -> Option<Vec<T>> {
        if let Some(par_index) = self.node_map.get(&id) {
            let mut descendant_idxs = HashSet::<usize>::new();

            let mut unexplored = VecDeque::<usize>::new();
            // Breadth first search of descendants.
            // Could make it depth first by changing between pop_front/back but this works for now.
            unexplored.push_back(par_index.clone());
            while let Some(idx) = unexplored.pop_front() {
                let children = &self.nodes.get(idx).unwrap().children;
                for child in children {
                    // Add to unexplored queue if:
                    // it's not already in it AND
                    // it's not already been explored (try to avoid loops)
                    if !unexplored.contains(&child) && !descendant_idxs.contains(&child) {
                        unexplored.push_back(*child)
                    }
                }
                descendant_idxs.extend(children); // extend consumes.
            }

            let descendants = descendant_idxs
                .iter()
                .map(|&idx| self.nodes.get(idx).unwrap().id.clone())
                .collect::<Vec<T>>();

            Some(descendants)
        } else {
            None
        }
    }

    pub fn ancestors(&self, id: &T) -> Option<Vec<T>> {
        if let Some(par_index) = self.node_map.get(&id) {
            let mut ancestor_idxs = HashSet::<usize>::new();

            let mut unexplored = VecDeque::<usize>::new();
            // Breadth first search of descendants.
            // Could make it depth first by changing between pop_front/back but this works for now.
            unexplored.push_back(par_index.clone());
            while let Some(idx) = unexplored.pop_front() {
                let parents = &self.nodes.get(idx).unwrap().parents;
                for parent in parents {
                    // Add to unexplored queue if:
                    if !unexplored.contains(&parent) // it's not already in it. 
                    && !ancestor_idxs.contains(&parent)
                    // It's not already been explored (try to avoid)
                    {
                        unexplored.push_back(*parent)
                    }
                }
                ancestor_idxs.extend(parents.clone()); // need to clone as extend consumes.
            }

            let ancestors = ancestor_idxs
                .iter()
                .map(|idx| self.nodes.get(*idx).unwrap().id.clone())
                .collect::<Vec<T>>();

            Some(ancestors)
        } else {
            None
        }
    }
}
