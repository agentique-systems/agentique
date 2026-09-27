//! The views of the Surface. History is a Panel, not a view.
use serde::{Deserialize, Serialize};

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum SurfaceView {
    /// Containment: what owns what, with connections between ports.
    #[default]
    Architecture,
    /// Layered by relationships: connections, typing, specialisation, satisfy.
    Graph,
    /// Requirements, what satisfies them and their subjects.
    Requirements,
}
impl SurfaceView {
    pub const ALL: [SurfaceView; 3] = [Self::Architecture, Self::Graph, Self::Requirements];
    pub fn title(self) -> &'static str {
        match self {
            Self::Architecture => "Architecture",
            Self::Graph => "Graph",
            Self::Requirements => "Requirements",
        }
    }
}
