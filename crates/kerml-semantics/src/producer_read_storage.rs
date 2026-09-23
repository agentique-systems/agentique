//! Exact immutable read sharing; allocation identity never enters closure truth.
use super::ProducerRead;
use std::collections::{BTreeSet, HashSet};
use std::sync::Arc;

/// Query capture remains contiguous. Only scheduler-retained rows pay for Arc
/// references, and their repeated atom payloads share one table-local interner.
#[derive(Clone)]
pub(crate) enum ProducerReads {
    Raw(Arc<[ProducerRead]>),
    Shared(Arc<[Arc<ProducerRead>]>),
}
impl From<Vec<ProducerRead>> for ProducerReads {
    fn from(reads: Vec<ProducerRead>) -> Self {
        Self::Raw(reads.into())
    }
}
impl std::fmt::Debug for ProducerReads {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}
impl PartialEq for ProducerReads {
    fn eq(&self, other: &Self) -> bool {
        self.iter().eq(other.iter())
    }
}
impl Eq for ProducerReads {}
impl ProducerReads {
    pub(crate) fn iter(&self) -> impl Iterator<Item = &ProducerRead> + Clone {
        let (raw, shared) = match self {
            Self::Raw(reads) => (Some(reads.iter()), None),
            Self::Shared(reads) => (None, Some(reads.iter().map(Arc::as_ref))),
        };
        raw.into_iter()
            .flatten()
            .chain(shared.into_iter().flatten())
    }
    #[cfg(test)]
    pub(crate) fn contains(&self, read: &ProducerRead) -> bool {
        self.iter().any(|candidate| candidate == read)
    }
    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Raw(reads) => reads.len(),
            Self::Shared(reads) => reads.len(),
        }
    }
}

/// Content-ordered entries avoid retaining a second copy of each enum as an
/// interner key. Certificates/checkpoints own rows, never this mutable pool.
#[derive(Debug, Default)]
pub(super) struct ProducerReadPool {
    atoms: BTreeSet<Arc<ProducerRead>>,
}
impl ProducerReadPool {
    pub(super) fn intern(&mut self, reads: &ProducerReads) -> ProducerReads {
        match reads {
            ProducerReads::Raw(_) => self.intern_values(reads.iter()),
            ProducerReads::Shared(atoms) => {
                let mut changed = false;
                let canonical: Vec<_> = atoms
                    .iter()
                    .map(|atom| {
                        if let Some(existing) = self.atoms.get(atom.as_ref()) {
                            changed |= !Arc::ptr_eq(existing, atom);
                            existing.clone()
                        } else {
                            self.atoms.insert(atom.clone());
                            atom.clone()
                        }
                    })
                    .collect();
                if changed {
                    ProducerReads::Shared(canonical.into())
                } else {
                    reads.clone()
                }
            }
        }
    }
    pub(super) fn intern_values<'a>(
        &mut self,
        reads: impl IntoIterator<Item = &'a ProducerRead>,
    ) -> ProducerReads {
        ProducerReads::Shared(
            reads
                .into_iter()
                .map(|read| {
                    if let Some(existing) = self.atoms.get(read) {
                        existing.clone()
                    } else {
                        let atom = Arc::new(read.clone());
                        self.atoms.insert(atom.clone());
                        atom
                    }
                })
                .collect(),
        )
    }
    pub(super) fn prune(&mut self) {
        self.atoms.retain(|atom| Arc::strong_count(atom) > 1);
    }
    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.atoms.len()
    }
}

/// Retained payload/reference bytes, excluding allocator and BTreeMap node
/// overhead. Unique live row/atom addresses are observed only for accounting.
#[derive(Debug, Default)]
pub(super) struct ReadStorage {
    pub logical_atoms: usize,
    pub unique_atoms: usize,
    pub row_bytes: usize,
    pub atom_bytes: usize,
    pub exclusion_bytes: usize,
}
impl ReadStorage {
    pub(super) fn observe<'a>(reads: impl IntoIterator<Item = &'a ProducerReads>) -> Self {
        let mut storage = Self::default();
        let mut rows = HashSet::new();
        let mut atoms = HashSet::new();
        let header_bytes = 2 * std::mem::size_of::<usize>();
        for reads in reads {
            storage.logical_atoms += reads.len();
            let (row, references) = match reads {
                ProducerReads::Raw(values) => (Arc::as_ptr(values).cast::<()>() as usize, 0),
                ProducerReads::Shared(values) => (
                    Arc::as_ptr(values).cast::<()>() as usize,
                    std::mem::size_of_val(values.as_ref()),
                ),
            };
            if !rows.insert(row) {
                continue;
            }
            storage.row_bytes += header_bytes + references;
            for read in reads.iter() {
                if atoms.insert(std::ptr::from_ref(read) as usize) {
                    storage.unique_atoms += 1;
                    storage.atom_bytes += std::mem::size_of::<ProducerRead>();
                    if matches!(reads, ProducerReads::Shared(_)) {
                        storage.atom_bytes += header_bytes;
                    }
                    if let ProducerRead::OwnedExcluding(_, _, excluded) = read {
                        storage.exclusion_bytes += std::mem::size_of_val(excluded.as_ref());
                    }
                }
            }
        }
        storage
    }
    pub(super) fn bytes(&self) -> usize {
        self.row_bytes + self.atom_bytes + self.exclusion_bytes
    }
}
