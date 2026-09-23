//! Exact, interned scheduler transport for separately authenticated frontiers.
//! These bytes are not a trusted publication receipt and expose no public restore.
use super::*;
use serde::ser::{SerializeSeq, SerializeStruct};
use serde::{Deserialize, Serialize};

type StoredReadRows = Vec<(ElementId, Vec<(usize, Vec<usize>)>)>;

#[derive(Serialize, Deserialize)]
pub(crate) struct FrontierCertificate {
    receipt: serde_json::Value,
    atoms: Vec<ProducerRead>,
    rows: StoredReadRows,
}

pub(crate) struct FrontierCertificateRef<'a>(&'a ProducerClosureCertificate);

impl Serialize for FrontierCertificateRef<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        // Only unique atoms need an index. The potentially large logical rows
        // remain borrowed and stream directly into the compressed output.
        let mut atoms = BTreeMap::<&ProducerRead, usize>::new();
        let mut ordered = Vec::new();
        for reads in self
            .0
            .transport_reads
            .values()
            .flat_map(|row| row.iter().map(|(_, reads)| reads))
        {
            for read in reads.iter() {
                let next = atoms.len();
                if let std::collections::btree_map::Entry::Vacant(entry) = atoms.entry(read) {
                    entry.insert(next);
                    ordered.push(read);
                }
            }
        }
        let mut state = serializer.serialize_struct("FrontierCertificate", 3)?;
        state.serialize_field("receipt", &self.0.receipt_value())?;
        state.serialize_field("atoms", &ordered)?;
        state.serialize_field(
            "rows",
            &BorrowedRows {
                rows: &self.0.transport_reads,
                atoms: &atoms,
            },
        )?;
        state.end()
    }
}

struct BorrowedRows<'a> {
    rows: &'a BTreeMap<ElementId, Vec<(usize, ProducerReads)>>,
    atoms: &'a BTreeMap<&'a ProducerRead, usize>,
}
impl Serialize for BorrowedRows<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.rows.len()))?;
        for (subject, row) in self.rows {
            sequence.serialize_element(&(
                subject,
                BorrowedFamilies {
                    row,
                    atoms: self.atoms,
                },
            ))?;
        }
        sequence.end()
    }
}
struct BorrowedFamilies<'a> {
    row: &'a [(usize, ProducerReads)],
    atoms: &'a BTreeMap<&'a ProducerRead, usize>,
}
impl Serialize for BorrowedFamilies<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.row.len()))?;
        for (family, reads) in self.row {
            sequence.serialize_element(&(
                family,
                BorrowedIndexes {
                    reads,
                    atoms: self.atoms,
                },
            ))?;
        }
        sequence.end()
    }
}
struct BorrowedIndexes<'a> {
    reads: &'a ProducerReads,
    atoms: &'a BTreeMap<&'a ProducerRead, usize>,
}
impl Serialize for BorrowedIndexes<'_> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut sequence = serializer.serialize_seq(Some(self.reads.len()))?;
        for read in self.reads.iter() {
            sequence.serialize_element(&self.atoms[read])?;
        }
        sequence.end()
    }
}

impl ProducerEvaluationTable {
    pub(crate) fn frontier_rows(&self) -> Vec<(ElementId, Vec<u8>)> {
        self.rows
            .iter()
            .map(|(&subject, states)| (subject, states.iter().map(|state| *state as u8).collect()))
            .collect()
    }

    pub(crate) fn restore_frontier_rows(
        &mut self,
        rows: Vec<(ElementId, Vec<u8>)>,
        registry: &ProducerRegistry,
        model: &ModelView,
    ) -> Option<()> {
        let mut restored = BTreeMap::new();
        for (subject, row) in rows {
            if model.element(subject).is_none() || row.len() != registry.descriptors().len() {
                return None;
            }
            let row = row
                .into_iter()
                .map(|state| match state {
                    0 => Some(ProducerEvaluationState::Inapplicable),
                    1 => Some(ProducerEvaluationState::Pending),
                    2 => Some(ProducerEvaluationState::EvaluatedComplete),
                    3 => Some(ProducerEvaluationState::EvaluatedIncomplete),
                    _ => None,
                })
                .collect::<Option<_>>()?;
            if restored.insert(subject, row).is_some() {
                return None;
            }
        }
        self.rows = restored;
        Some(())
    }
}

impl ProducerClosureCertificate {
    /// Exact negative/provider/search transport identity, independent of Arc
    /// allocation and distinct from the compact semantic closure receipt.
    pub fn revalidation_digest(&self) -> [u8; 32] {
        struct HashWriter(Sha256);
        impl std::io::Write for HashWriter {
            fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
                self.0.update(bytes);
                Ok(bytes.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
                Ok(())
            }
        }
        let mut hash = Sha256::new();
        hash.update(b"agq-producer-transport/1\0");
        let mut atoms = std::collections::HashMap::new();
        for (subject, families) in self.transport_reads.iter() {
            hash.update(subject.as_u128().to_le_bytes());
            hash.update((families.len() as u64).to_le_bytes());
            for (family, reads) in families {
                hash.update((*family as u64).to_le_bytes());
                hash.update((reads.len() as u64).to_le_bytes());
                for read in reads.iter() {
                    // Addresses accelerate repeated immutable atoms; only their
                    // semantic bytes enter the digest, including unshared rows.
                    let digest: &[u8; 32] = atoms
                        .entry(std::ptr::from_ref(read) as usize)
                        .or_insert_with(|| {
                            let mut writer = HashWriter(Sha256::new());
                            serde_json::to_writer(&mut writer, read)
                                .expect("serializable producer read");
                            writer.0.finalize().into()
                        });
                    hash.update(digest);
                }
            }
        }
        hash.finalize().into()
    }
    pub(crate) fn frontier_state(&self) -> FrontierCertificateRef<'_> {
        FrontierCertificateRef(self)
    }

    pub(crate) fn restore_frontier_state(
        state: FrontierCertificate,
        context: &SemanticContextId,
        registry: &ProducerRegistry,
        model: &ModelView,
    ) -> Option<Self> {
        let mut certificate = Self::from_trusted_receipt(&state.receipt, context, registry, model)?;
        let atoms: Vec<_> = state.atoms.into_iter().map(Arc::new).collect();
        let mut reads = BTreeMap::new();
        for (subject, families) in state.rows {
            if model.element(subject).is_none() || reads.contains_key(&subject) {
                return None;
            }
            let mut seen = BTreeSet::new();
            let mut row = Vec::new();
            for (family, indexes) in families {
                if family >= registry.descriptors().len() || !seen.insert(family) {
                    return None;
                }
                let values: Vec<_> = indexes
                    .into_iter()
                    .map(|index| atoms.get(index).cloned())
                    .collect::<Option<_>>()?;
                if !values.windows(2).all(|pair| pair[0] < pair[1]) {
                    return None;
                }
                row.push((family, ProducerReads::Shared(values.into())));
            }
            reads.insert(subject, row);
        }
        certificate.transport_reads = Arc::new(reads);
        Some(certificate)
    }
}
