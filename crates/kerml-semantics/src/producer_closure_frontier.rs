//! Exact, interned scheduler transport for separately authenticated frontiers.
//! These bytes are not a trusted publication receipt and expose no public restore.
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct FrontierCertificate {
    receipt: serde_json::Value,
    atoms: Vec<ProducerRead>,
    rows: Vec<(ElementId, Vec<(usize, Vec<usize>)>)>,
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
        let state = self.frontier_state();
        Sha256::digest(
            serde_json::to_vec(&(state.atoms, state.rows)).expect("serializable producer reads"),
        )
        .into()
    }
    pub(crate) fn frontier_state(&self) -> FrontierCertificate {
        let mut atoms = BTreeMap::<&ProducerRead, usize>::new();
        let rows = self
            .transport_reads
            .iter()
            .map(|(&subject, families)| {
                let families = families
                    .iter()
                    .map(|(family, reads)| {
                        let reads = reads
                            .iter()
                            .map(|read| {
                                let next = atoms.len();
                                *atoms.entry(read).or_insert(next)
                            })
                            .collect();
                        (*family, reads)
                    })
                    .collect();
                (subject, families)
            })
            .collect();
        let mut ordered: Vec<_> = atoms
            .into_iter()
            .map(|(read, index)| (index, read.clone()))
            .collect();
        ordered.sort_by_key(|(index, _)| *index);
        FrontierCertificate {
            receipt: self.receipt_value(),
            atoms: ordered.into_iter().map(|(_, read)| read).collect(),
            rows,
        }
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
