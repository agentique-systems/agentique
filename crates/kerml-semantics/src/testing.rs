//! Verification-only observations of actual scheduler evaluation events.
use agq_kernel::ElementId;
use std::{cell::RefCell, collections::BTreeSet, marker::PhantomData, rc::Rc};
thread_local! { static SUBJECTS: RefCell<Vec<BTreeSet<ElementId>>> = const { RefCell::new(Vec::new()) }; }
/// Scope-bound per-thread recording. Nested observations retain their own events.
pub struct ProducerObservation {
    depth: usize,
    active: bool,
    thread: PhantomData<Rc<()>>,
}
impl ProducerObservation {
    pub fn start() -> Self {
        let depth = SUBJECTS.with(|events| {
            let mut events = events.borrow_mut();
            let depth = events.len();
            events.push(BTreeSet::new());
            depth
        });
        Self {
            depth,
            active: true,
            thread: PhantomData,
        }
    }
    pub fn finish(mut self) -> BTreeSet<ElementId> {
        SUBJECTS.with(|events| {
            let mut events = events.borrow_mut();
            assert_eq!(
                events.len(),
                self.depth + 1,
                "observation scopes must finish in order"
            );
            self.active = false;
            events.pop().expect("active observation")
        })
    }
}
impl Drop for ProducerObservation {
    fn drop(&mut self) {
        if self.active {
            SUBJECTS.with(|events| {
                events.borrow_mut().truncate(self.depth);
            });
        }
    }
}
pub(crate) fn record(subject: ElementId) {
    SUBJECTS.with(|events| {
        for observed in &mut *events.borrow_mut() {
            observed.insert(subject);
        }
    });
}
