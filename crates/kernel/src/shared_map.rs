//! Persistent immutable base tables with independently mutable local rows.
//!
//! Kernel dependency guards authorize writes; this container only preserves
//! storage ownership. A fork borrows all inherited rows and starts an empty delta.
use std::collections::{BTreeMap, btree_map};
use std::ops::Index;
use std::sync::Arc;

#[derive(Clone, Debug)]
pub(crate) struct SharedMap<K, V> {
    base: Option<Arc<Self>>,
    local: Arc<BTreeMap<K, V>>,
}

impl<K, V> Default for SharedMap<K, V> {
    fn default() -> Self {
        Self {
            base: None,
            local: Arc::new(BTreeMap::new()),
        }
    }
}

impl<K: Ord, V> SharedMap<K, V> {
    pub(crate) fn new() -> Self {
        Self::default()
    }
    pub(crate) fn get(&self, key: &K) -> Option<&V> {
        self.local.get(key).or_else(|| self.base.as_ref()?.get(key))
    }
    pub(crate) fn contains_key(&self, key: &K) -> bool {
        self.get(key).is_some()
    }
    pub(crate) fn len(&self) -> usize {
        self.local.len()
            + self.base.as_ref().map_or(0, |base| {
                base.len()
                    - self
                        .local
                        .keys()
                        .filter(|key| base.contains_key(key))
                        .count()
            })
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.local.is_empty() && self.base.as_ref().is_none_or(|base| base.is_empty())
    }
    pub(crate) fn iter(&self) -> Iter<'_, K, V> {
        Iter {
            local: self.local.iter().peekable(),
            base: self
                .base
                .as_ref()
                .map(|base| Box::new(base.iter()).peekable()),
        }
    }
    pub(crate) fn keys(&self) -> impl Iterator<Item = &K> {
        self.iter().map(|(key, _)| key)
    }
    pub(crate) fn values(&self) -> impl Iterator<Item = &V> {
        self.iter().map(|(_, value)| value)
    }
    pub(crate) fn local_iter(&self) -> btree_map::Iter<'_, K, V> {
        self.local.iter()
    }
    pub(crate) fn local_values(&self) -> btree_map::Values<'_, K, V> {
        self.local.values()
    }
    #[cfg(any(test, feature = "verification"))]
    pub(crate) fn local_table_id(&self) -> usize {
        Arc::as_ptr(&self.local) as usize
    }
    #[cfg(any(test, feature = "verification"))]
    pub(crate) fn base(&self) -> Option<&Self> {
        self.base.as_deref()
    }
}

impl<K: Ord + Clone, V: Clone> SharedMap<K, V> {
    pub(crate) fn into_local(self) -> BTreeMap<K, V> {
        assert!(
            self.base.is_none(),
            "cannot flatten an immutable base table"
        );
        Arc::try_unwrap(self.local).unwrap_or_else(|local| (*local).clone())
    }
    pub(crate) fn fork(&self) -> Self {
        Self {
            base: Some(Arc::new(self.clone())),
            local: Arc::new(BTreeMap::new()),
        }
    }
    pub(crate) fn replace_base(mut self, base: &Self) -> Self {
        self.base = Some(Arc::new(base.clone()));
        self
    }
    pub(crate) fn with_base(mut self, base: &Self) -> Self {
        assert!(self.base.is_none(), "table already has an immutable base");
        self.base = Some(Arc::new(base.clone()));
        self
    }
    pub(crate) fn insert(&mut self, key: K, value: V) -> Option<V> {
        Arc::make_mut(&mut self.local).insert(key, value)
    }
    /// Dependency rows are protected by the model. Mutable lookup never copies
    /// an inherited value into the delta as an incidental read.
    pub(crate) fn get_mut(&mut self, key: &K) -> Option<&mut V> {
        Arc::make_mut(&mut self.local).get_mut(key)
    }
    pub(crate) fn remove(&mut self, key: &K) -> Option<V> {
        Arc::make_mut(&mut self.local).remove(key)
    }
    pub(crate) fn entry(&mut self, key: K) -> btree_map::Entry<'_, K, V> {
        Arc::make_mut(&mut self.local).entry(key)
    }
    pub(crate) fn values_mut(&mut self) -> btree_map::ValuesMut<'_, K, V> {
        Arc::make_mut(&mut self.local).values_mut()
    }
    pub(crate) fn iter_mut(&mut self) -> btree_map::IterMut<'_, K, V> {
        Arc::make_mut(&mut self.local).iter_mut()
    }
    pub(crate) fn retain(&mut self, mut keep: impl FnMut(&K, &mut V) -> bool) {
        // Only local rows are editable; dependency protection is never bypassed
        // by a metadata filter. Callers filter selected local fact populations.
        Arc::make_mut(&mut self.local).retain(|key, value| keep(key, value));
    }
}

impl<K: Ord + Clone, V: Clone> Extend<(K, V)> for SharedMap<K, V> {
    fn extend<T: IntoIterator<Item = (K, V)>>(&mut self, entries: T) {
        Arc::make_mut(&mut self.local).extend(entries);
    }
}
impl<K: Ord, V> FromIterator<(K, V)> for SharedMap<K, V> {
    fn from_iter<T: IntoIterator<Item = (K, V)>>(entries: T) -> Self {
        Self {
            base: None,
            local: Arc::new(entries.into_iter().collect()),
        }
    }
}
impl<K: Ord, V> From<BTreeMap<K, V>> for SharedMap<K, V> {
    fn from(local: BTreeMap<K, V>) -> Self {
        Self {
            base: None,
            local: Arc::new(local),
        }
    }
}
impl<K: Ord, V: PartialEq> PartialEq for SharedMap<K, V> {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}
impl<K: Ord, V: Eq> Eq for SharedMap<K, V> {}
impl<K: Ord, V> Index<&K> for SharedMap<K, V> {
    type Output = V;
    fn index(&self, key: &K) -> &V {
        self.get(key).expect("validated table key")
    }
}
impl<'a, K: Ord, V> IntoIterator for &'a SharedMap<K, V> {
    type Item = (&'a K, &'a V);
    type IntoIter = Iter<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}
impl<'a, K: Ord + Clone, V: Clone> IntoIterator for &'a mut SharedMap<K, V> {
    type Item = (&'a K, &'a mut V);
    type IntoIter = btree_map::IterMut<'a, K, V>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter_mut()
    }
}

pub(crate) struct Iter<'a, K: Ord, V> {
    local: std::iter::Peekable<btree_map::Iter<'a, K, V>>,
    base: Option<std::iter::Peekable<Box<Self>>>,
}
impl<'a, K: Ord, V> Iterator for Iter<'a, K, V> {
    type Item = (&'a K, &'a V);
    fn next(&mut self) -> Option<Self::Item> {
        let Some(base) = self.base.as_mut() else {
            return self.local.next();
        };
        match (self.local.peek(), base.peek()) {
            (Some((local, _)), Some((inherited, _))) => match local.cmp(inherited) {
                std::cmp::Ordering::Less => self.local.next(),
                std::cmp::Ordering::Greater => base.next(),
                std::cmp::Ordering::Equal => {
                    base.next();
                    self.local.next()
                }
            },
            (Some(_), None) => self.local.next(),
            (None, _) => base.next(),
        }
    }
}

pub(crate) struct Merge<'a, T> {
    left: std::iter::Peekable<Box<dyn Iterator<Item = &'a T> + 'a>>,
    right: std::iter::Peekable<Box<dyn Iterator<Item = &'a T> + 'a>>,
    compare: fn(&T, &T) -> std::cmp::Ordering,
}
pub(crate) fn merge<'a, T>(
    left: impl Iterator<Item = &'a T> + 'a,
    right: impl Iterator<Item = &'a T> + 'a,
    compare: fn(&T, &T) -> std::cmp::Ordering,
) -> Merge<'a, T> {
    Merge {
        left: (Box::new(left) as Box<dyn Iterator<Item = &'a T>>).peekable(),
        right: (Box::new(right) as Box<dyn Iterator<Item = &'a T>>).peekable(),
        compare,
    }
}
impl<'a, T> Iterator for Merge<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<Self::Item> {
        match (self.left.peek(), self.right.peek()) {
            (Some(left), Some(right)) => match (self.compare)(left, right) {
                std::cmp::Ordering::Less => self.left.next(),
                std::cmp::Ordering::Greater => self.right.next(),
                std::cmp::Ordering::Equal => {
                    self.left.next();
                    self.right.next()
                }
            },
            (Some(_), None) => self.left.next(),
            (None, _) => self.right.next(),
        }
    }
}

/// Persistent identity reservations, including removed local identities.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SharedSet<K: Ord>(pub(crate) SharedMap<K, ()>);
impl<K: Ord + Clone> SharedSet<K> {
    pub(crate) fn new() -> Self {
        Self(SharedMap::new())
    }
    pub(crate) fn contains(&self, key: &K) -> bool {
        self.0.contains_key(key)
    }
    pub(crate) fn iter(&self) -> impl Iterator<Item = &K> {
        self.0.keys()
    }
    pub(crate) fn fork(&self) -> Self {
        Self(self.0.fork())
    }
    pub(crate) fn insert(&mut self, key: K) -> bool {
        if self.contains(&key) {
            return false;
        }
        self.0.insert(key, ());
        true
    }
}
impl<K: Ord + Clone> Extend<K> for SharedSet<K> {
    fn extend<T: IntoIterator<Item = K>>(&mut self, entries: T) {
        for key in entries {
            self.insert(key);
        }
    }
}
impl<K: Ord + Clone> FromIterator<K> for SharedSet<K> {
    fn from_iter<T: IntoIterator<Item = K>>(entries: T) -> Self {
        Self(entries.into_iter().map(|key| (key, ())).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn forks_merge_in_identity_order_and_mutate_only_the_local_delta() {
        let base: SharedMap<_, _> = [(1, "a"), (3, "c"), (5, "e")].into_iter().collect();
        let mut fork = base.fork();
        fork.insert(2, "b");
        fork.insert(3, "local c");
        assert!(fork.get_mut(&1).is_none());
        assert!(fork.remove(&5).is_none());
        assert_eq!(
            fork.iter().map(|(k, v)| (*k, *v)).collect::<Vec<_>>(),
            [(1, "a"), (2, "b"), (3, "local c"), (5, "e")]
        );
        assert_eq!(fork.len(), 4);
        assert_eq!(base[&3], "c");
        let earlier = fork.clone();
        fork.insert(4, "d");
        assert!(!earlier.contains_key(&4));
        assert_eq!(fork.base().unwrap().local_table_id(), base.local_table_id());
        assert_eq!(
            earlier.base().unwrap().local_table_id(),
            base.local_table_id()
        );
    }
}
