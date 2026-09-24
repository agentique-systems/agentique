#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Point {
    AfterSourceBlobWrite,
    AfterRevisionManifestWrite,
    BeforeHeadUpdate,
    AfterHeadUpdateStatement,
    BeforeTransactionCommit,
    AfterDurableCommit,
}

#[derive(Default)]
pub(crate) struct Faults {
    #[cfg(test)]
    action: std::sync::Mutex<Option<(Point, bool)>>,
}

impl Faults {
    pub(crate) fn check(&self, point: Point) -> Result<(), String> {
        #[cfg(test)]
        {
            let mut action = self.action.lock().expect("fault mutex");
            if let Some((selected, abort_process)) = *action
                && selected == point
            {
                *action = None;
                if abort_process {
                    std::process::exit(86);
                }
                return Err(format!("injected failure: {point:?}"));
            }
        }
        let _ = point;
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn set(&self, point: Point, abort_process: bool) {
        *self.action.lock().expect("fault mutex") = Some((point, abort_process));
    }
}
