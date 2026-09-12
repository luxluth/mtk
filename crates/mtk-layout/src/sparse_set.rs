pub const SPARSE_NULL: u32 = u32::MAX;

#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct NodeId {
    pub index: u32,
    pub generation: u32,
}

impl NodeId {
    pub const INVALID: NodeId = NodeId {
        index: u32::MAX,
        generation: u32::MAX,
    };

    #[inline(always)]
    pub fn is_valid(self) -> bool {
        self.index != u32::MAX && self.generation != u32::MAX
    }
}

/// Cache-friendly Data-Oriented Sparse Set mapping `NodeId`s to contiguous components.
#[derive(Clone, Debug)]
pub struct SparseSet<T> {
    sparse: Vec<u32>,
    dense: Vec<NodeId>,
    values: Vec<T>,
}

impl<T> Default for SparseSet<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> SparseSet<T> {
    pub fn new() -> Self {
        Self {
            sparse: Vec::new(),
            dense: Vec::new(),
            values: Vec::new(),
        }
    }

    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            sparse: Vec::with_capacity(capacity),
            dense: Vec::with_capacity(capacity),
            values: Vec::with_capacity(capacity),
        }
    }

    #[inline(always)]
    pub fn has(&self, id: NodeId) -> bool {
        let idx = id.index as usize;
        if idx < self.sparse.len() {
            let dense_idx = unsafe { *self.sparse.get_unchecked(idx) };
            if dense_idx != SPARSE_NULL && (dense_idx as usize) < self.dense.len() {
                let entity = unsafe { *self.dense.get_unchecked(dense_idx as usize) };
                return entity.generation == id.generation;
            }
        }
        false
    }

    #[inline(always)]
    pub fn get(&self, id: NodeId) -> Option<&T> {
        if self.has(id) {
            let dense_idx = unsafe { *self.sparse.get_unchecked(id.index as usize) } as usize;
            Some(unsafe { self.values.get_unchecked(dense_idx) })
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut T> {
        if self.has(id) {
            let dense_idx = unsafe { *self.sparse.get_unchecked(id.index as usize) } as usize;
            Some(unsafe { self.values.get_unchecked_mut(dense_idx) })
        } else {
            None
        }
    }

    /// # Safety
    /// Will panic if the NodeId doesn't have component this component. Use [SparseSet::has] as a safety check
    #[inline(always)]
    pub unsafe fn get_unchecked(&self, id: NodeId) -> &T {
        debug_assert!(self.has(id));
        unsafe {
            let dense_idx = *self.sparse.get_unchecked(id.index as usize) as usize;
            self.values.get_unchecked(dense_idx)
        }
    }

    /// # Safety
    /// Will panic if the NodeId doesn't have component this component. Use [SparseSet::has] as a safety check
    #[inline(always)]
    pub unsafe fn get_unchecked_mut(&mut self, id: NodeId) -> &mut T {
        debug_assert!(self.has(id));
        unsafe {
            let dense_idx = *self.sparse.get_unchecked(id.index as usize) as usize;
            self.values.get_unchecked_mut(dense_idx)
        }
    }

    #[inline]
    pub fn insert(&mut self, id: NodeId, value: T) {
        let idx = id.index as usize;
        if idx >= self.sparse.len() {
            self.sparse.resize(idx + 1, SPARSE_NULL);
        }

        let dense_idx = self.sparse[idx];
        if dense_idx == SPARSE_NULL {
            let new_dense_idx = self.dense.len() as u32;
            self.sparse[idx] = new_dense_idx;
            self.dense.push(id);
            self.values.push(value);
        } else {
            let d_idx = dense_idx as usize;
            self.dense[d_idx] = id;
            self.values[d_idx] = value;
        }
    }

    #[inline]
    pub fn remove(&mut self, id: NodeId) -> Option<T> {
        if self.has(id) {
            let dense_idx = self.sparse[id.index as usize] as usize;
            let last_idx = self.dense.len() - 1;
            let last_entity = self.dense[last_idx];

            self.dense[dense_idx] = last_entity;
            let removed = self.values.swap_remove(dense_idx);
            self.dense.pop();

            self.sparse[last_entity.index as usize] = dense_idx as u32;
            self.sparse[id.index as usize] = SPARSE_NULL;

            Some(removed)
        } else {
            None
        }
    }

    #[inline]
    pub fn clear(&mut self) {
        for id in &self.dense {
            let idx = id.index as usize;
            if idx < self.sparse.len() {
                self.sparse[idx] = SPARSE_NULL;
            }
        }
        self.dense.clear();
        self.values.clear();
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        self.dense.len()
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.dense.is_empty()
    }

    #[inline(always)]
    pub fn dense(&self) -> &[NodeId] {
        &self.dense
    }

    #[inline(always)]
    pub fn values(&self) -> &[T] {
        &self.values
    }

    #[inline(always)]
    pub fn values_mut(&mut self) -> &mut [T] {
        &mut self.values
    }

    pub fn iter(&self) -> impl Iterator<Item = (NodeId, &T)> {
        self.dense.iter().copied().zip(self.values.iter())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = (NodeId, &mut T)> {
        self.dense.iter().copied().zip(self.values.iter_mut())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparse_set_lifecycle() {
        let mut set = SparseSet::<String>::new();
        let n1 = NodeId {
            index: 0,
            generation: 0,
        };
        let n2 = NodeId {
            index: 1,
            generation: 0,
        };
        let n3 = NodeId {
            index: 2,
            generation: 0,
        };

        set.insert(n1, "A".to_string());
        set.insert(n2, "B".to_string());
        set.insert(n3, "C".to_string());

        assert_eq!(set.len(), 3);
        assert_eq!(set.get(n1), Some(&"A".to_string()));
        assert_eq!(set.get(n2), Some(&"B".to_string()));
        assert_eq!(set.get(n3), Some(&"C".to_string()));

        // Remove middle element (n2)
        assert_eq!(set.remove(n2), Some("B".to_string()));
        assert!(!set.has(n2));
        assert_eq!(set.get(n2), None);
        assert_eq!(set.len(), 2);

        // n1 and n3 must remain intact
        assert_eq!(set.get(n1), Some(&"A".to_string()));
        assert_eq!(set.get(n3), Some(&"C".to_string()));

        // Outdated generation check
        let n1_old = NodeId {
            index: 0,
            generation: 1,
        };
        assert!(!set.has(n1_old));
    }
}
