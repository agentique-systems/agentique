//! Bounded surface recovery through eframe's supported hook. Device replacement
//! remains renderer-owned in eframe 0.36; a lost device is an explicit stop state.
use eframe::egui;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Presentation persistence is separate from renderer recovery and model writes.
/// A skipped (unstable) view is retried as soon as it becomes saveable; an I/O
/// failure is retained independently of the status text and retried at most every
/// eight seconds, so a failed save cannot cause a write loop on each frame.
#[derive(Default)]
pub(crate) struct SavedLayout {
    fault: Option<String>,
    last_attempt: Option<Instant>,
    result: Option<Result<(), String>>,
}

impl SavedLayout {
    pub fn needs_attempt(&mut self, message: &str) -> bool {
        if self.fault.as_deref() != Some(message) {
            *self = Self {
                fault: Some(message.to_owned()),
                ..Self::default()
            };
        }
        self.last_attempt
            .is_none_or(|attempt| attempt.elapsed() >= Duration::from_secs(8))
    }

    pub fn record(&mut self, result: Result<bool, String>) {
        if result == Ok(false) {
            return;
        }
        self.last_attempt = Some(Instant::now());
        self.result = Some(result.map(|_| ()));
    }

    pub fn failed(&self) -> bool {
        matches!(self.result, Some(Err(_)))
    }

    pub fn status(&self, message: &str) -> String {
        match &self.result {
            Some(Ok(())) => format!("{message} The layout was saved."),
            Some(Err(error)) => format!(
                "{message} The layout was not saved: {error}. Restart restores the last saved layout."
            ),
            None => {
                format!("{message} Saving the layout is waiting.")
            }
        }
    }
}

pub(crate) fn checkpoint_title(context: &egui::Context, device: bool, failed: bool) {
    // The OS title remains readable even when the GPU cannot draw the notice.
    let title = if failed {
        "Agentique · Graphics unavailable — layout not saved; restart required"
    } else if device {
        "Agentique · Graphics device lost — restart required"
    } else {
        "Agentique · Graphics surface unavailable — resize or restart"
    };
    context.send_viewport_cmd(egui::ViewportCommand::Title(title.into()));
}

#[derive(Clone, Default)]
pub struct Recovery(Arc<Mutex<State>>);

#[derive(Default)]
struct State {
    epoch: u64,
    failures: usize,
    blocked: Option<String>,
    device_lost: bool,
}

/// Why acquiring a frame failed: wgpu's `CurrentSurfaceTexture` without the
/// texture, so it can be copied and tested.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceFailure {
    Timeout,
    /// The window is hidden (minimised or covered); not a failure.
    Occluded,
    Outdated,
    Lost,
    Validation,
}

impl SurfaceFailure {
    fn from_status(status: &wgpu::CurrentSurfaceTexture) -> Option<Self> {
        use wgpu::CurrentSurfaceTexture as Status;
        match status {
            Status::Success(_) | Status::Suboptimal(_) => None,
            Status::Timeout => Some(Self::Timeout),
            Status::Occluded => Some(Self::Occluded),
            Status::Outdated => Some(Self::Outdated),
            Status::Lost => Some(Self::Lost),
            Status::Validation => Some(Self::Validation),
        }
    }
}

impl std::fmt::Display for SurfaceFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Timeout => "the frame timed out",
            Self::Occluded => "the window is hidden",
            Self::Outdated => "the surface is outdated",
            Self::Lost => "the surface was lost",
            Self::Validation => "a validation error",
        })
    }
}

struct Response {
    action: egui_wgpu::SurfaceErrorAction,
    retry: Option<Duration>,
    message: Option<String>,
}

impl State {
    fn error(&mut self, error: SurfaceFailure) -> Response {
        if error == SurfaceFailure::Occluded {
            // A hidden window draws nothing; showing it again resumes.
            return Response {
                action: egui_wgpu::SurfaceErrorAction::SkipFrame,
                retry: None,
                message: None,
            };
        }
        self.epoch += 1;
        if self.device_lost {
            return Response {
                action: egui_wgpu::SurfaceErrorAction::SkipFrame,
                retry: None,
                message: None,
            };
        }
        self.failures += 1;
        let recoverable = matches!(
            error,
            SurfaceFailure::Lost | SurfaceFailure::Outdated | SurfaceFailure::Timeout
        );
        let retry = recoverable
            .then(|| [16, 50, 100, 250, 500].get(self.failures - 1).copied())
            .flatten()
            .map(Duration::from_millis);
        if retry.is_none() {
            self.blocked = Some(format!(
                "Graphics surface unavailable ({error}); resize the window or restart Native Studio. Model history is unchanged."
            ));
        }
        Response {
            // eframe calls Surface::configure here; it does not replace a device.
            action: if matches!(error, SurfaceFailure::Lost | SurfaceFailure::Outdated) {
                egui_wgpu::SurfaceErrorAction::RecreateSurface
            } else {
                egui_wgpu::SurfaceErrorAction::SkipFrame
            },
            retry,
            message: (self.failures == 1 || (recoverable && self.failures == 6)).then(|| {
                self.blocked
                    .clone()
                    .unwrap_or_else(|| format!("Graphics surface recovery: {error}"))
            }),
        }
    }
}

impl Recovery {
    pub fn attach(&self, context: &egui::Context, device: &wgpu::Device) {
        context.data_mut(|data| {
            data.insert_temp(egui::Id::new("native-surface-recovery"), self.clone())
        });
        let recovery = self.clone();
        let context = context.clone();
        device.set_device_lost_callback(move |reason, message| {
            recovery.device_lost(reason, &message, Some(&context));
        });
    }

    /// eframe's `on_surface_status` hook: called when a frame could not be
    /// acquired.
    pub fn surface_status(
        &self,
        status: &wgpu::CurrentSurfaceTexture,
        context: Option<&egui::Context>,
    ) -> egui_wgpu::SurfaceErrorAction {
        match SurfaceFailure::from_status(status) {
            Some(failure) => self.surface_error(failure, context),
            None => egui_wgpu::SurfaceErrorAction::SkipFrame,
        }
    }

    pub fn surface_error(
        &self,
        error: SurfaceFailure,
        context: Option<&egui::Context>,
    ) -> egui_wgpu::SurfaceErrorAction {
        let response = self.0.lock().expect("surface recovery state").error(error);
        if let Some(message) = &response.message {
            eprintln!("{message}");
        }
        if let Some(context) = context {
            if let Some(delay) = response.retry {
                // A skipped acquisition otherwise may leave an idle window asleep.
                context.request_repaint_after(delay);
            } else if response.message.is_some() {
                context.send_viewport_cmd(egui::ViewportCommand::Title(
                    "Agentique · Graphics surface unavailable — resize or restart".into(),
                ));
                context.request_repaint();
            }
        }
        response.action
    }

    pub(crate) fn device_lost(
        &self,
        reason: wgpu::DeviceLostReason,
        detail: &str,
        context: Option<&egui::Context>,
    ) {
        if reason == wgpu::DeviceLostReason::Destroyed {
            return; // Normal device teardown is not a recovery incident.
        }
        let message = format!(
            "Graphics device lost ({reason:?}): {detail}. Restart the Studio to recreate the renderer; the saved model is not affected."
        );
        {
            let mut state = self.0.lock().expect("surface recovery state");
            state.device_lost = true;
            state.epoch += 1;
            state.blocked = Some(message.clone());
        }
        eprintln!("{message}");
        if let Some(context) = context {
            // The OS title remains available when the GPU can no longer draw text.
            context.send_viewport_cmd(egui::ViewportCommand::Title(
                "Agentique · Graphics device lost — restart required".into(),
            ));
            context.request_repaint();
        }
    }

    pub fn epoch(&self) -> u64 {
        self.0.lock().expect("surface recovery state").epoch
    }

    pub fn device_unavailable(&self) -> bool {
        self.0.lock().expect("surface recovery state").device_lost
    }

    fn acquired(&self, context: &egui::Context) {
        let recovered = {
            let mut state = self.0.lock().expect("surface recovery state");
            if state.device_lost {
                return;
            }
            state.failures = 0;
            state.blocked.take().is_some()
        };
        if recovered {
            context.send_viewport_cmd(egui::ViewportCommand::Title(
                "Agentique · Native Studio".into(),
            ));
            context.request_repaint();
            eprintln!("Graphics surface acquired again; rendering resumed.");
        }
    }
}

fn for_context(context: &egui::Context) -> Option<Recovery> {
    context.data(|data| data.get_temp(egui::Id::new("native-surface-recovery")))
}

pub fn device_fault(context: &egui::Context) -> Option<String> {
    let recovery = for_context(context)?;
    let state = recovery.0.lock().ok()?;
    state.device_lost.then(|| state.blocked.clone()).flatten()
}

pub fn surface_fault(context: &egui::Context) -> Option<String> {
    let recovery = for_context(context)?;
    let state = recovery.0.lock().ok()?;
    (!state.device_lost)
        .then(|| state.blocked.clone())
        .flatten()
}

/// This callback runs only after successful surface acquisition, including setup
/// screens without a semantic scene. It is not evidence of physical presentation.
pub fn paint_heartbeat(context: &egui::Context) {
    if let Some(recovery) = for_context(context) {
        context.layer_painter(egui::LayerId::background()).add(
            egui_wgpu::Callback::new_paint_callback(
                context.content_rect(),
                Heartbeat {
                    recovery,
                    context: context.clone(),
                },
            ),
        );
    }
}

struct Heartbeat {
    recovery: Recovery,
    context: egui::Context,
}
impl egui_wgpu::CallbackTrait for Heartbeat {
    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        _pass: &mut wgpu::RenderPass<'static>,
        _resources: &egui_wgpu::CallbackResources,
    ) {
        self.recovery.acquired(&self.context);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lost_surface_reconfigures_and_schedules_only_five_automatic_retries() {
        let mut state = State::default();
        for delay in [16, 50, 100, 250, 500] {
            let response = state.error(SurfaceFailure::Lost);
            assert!(matches!(
                response.action,
                egui_wgpu::SurfaceErrorAction::RecreateSurface
            ));
            assert_eq!(response.retry, Some(Duration::from_millis(delay)));
        }
        assert!(state.error(SurfaceFailure::Lost).retry.is_none());
        assert!(state.blocked.as_ref().unwrap().contains("resize"));
        assert!(state.error(SurfaceFailure::Lost).message.is_none());
    }
    #[test]
    fn timeout_skips_without_reconfigure_and_validation_never_auto_retries() {
        let timeout = State::default().error(SurfaceFailure::Timeout);
        assert!(matches!(
            timeout.action,
            egui_wgpu::SurfaceErrorAction::SkipFrame
        ));
        assert!(timeout.retry.is_some());
        let validation = State::default().error(SurfaceFailure::Validation);
        assert!(matches!(
            validation.action,
            egui_wgpu::SurfaceErrorAction::SkipFrame
        ));
        assert!(validation.retry.is_none());
        let hidden = State::default().error(SurfaceFailure::Occluded);
        assert!(hidden.retry.is_none() && hidden.message.is_none());
    }
    #[test]
    fn surface_acquisition_resets_retry_budget_but_cannot_reset_a_lost_device() {
        let recovery = Recovery::default();
        recovery.surface_error(SurfaceFailure::Lost, None);
        let epoch = recovery.epoch();
        recovery.acquired(&egui::Context::default());
        assert_eq!(recovery.0.lock().unwrap().failures, 0);
        assert_eq!(
            recovery.epoch(),
            epoch,
            "timestamps must still see the failed acquisition"
        );
        recovery.device_lost(
            wgpu::DeviceLostReason::Unknown,
            "injected device-loss callback",
            None,
        );
        recovery.acquired(&egui::Context::default());
        assert!(recovery.device_unavailable());
        assert!(matches!(
            recovery.surface_error(SurfaceFailure::Lost, None),
            egui_wgpu::SurfaceErrorAction::SkipFrame
        ));
        let state = recovery.0.lock().unwrap();
        assert!(state.blocked.as_ref().unwrap().contains("Restart"));
    }
    #[test]
    fn normal_device_teardown_does_not_report_loss() {
        let recovery = Recovery::default();
        recovery.device_lost(wgpu::DeviceLostReason::Destroyed, "normal shutdown", None);
        assert!(!recovery.device_unavailable());
        assert_eq!(recovery.epoch(), 0);
    }
}
