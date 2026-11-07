//! A saved Light scene: everything needed to show it again — the panel
//! settings (scene type, placement, quality, sensors), the plan of a
//! planned scene, the objects moved by hand, the sensor positions, the
//! camera, and which sensor file and luminaire it was made with (by name /
//! project item id; the files themselves live in the project).
//!
//! Serialised as JSON by the host (`serde`); `Viewer::scene_doc` captures
//! one, `Viewer::apply_doc` shows it.

use serde::{Deserialize, Serialize};

use crate::scene::{CameraView, ObjectId, ScenePlan, SceneSensor};
use crate::sync::SyncSettings;

/// Format version of [`SceneDoc`].
pub const SCENE_DOC_VERSION: u32 = 1;

fn version() -> u32 {
    SCENE_DOC_VERSION
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SceneDoc {
    #[serde(default = "version")]
    pub version: u32,
    /// Stable id: the project item of a saved scene is `scene-<id>`.
    pub id: String,
    pub name: String,
    /// The panel settings (`scene` names the scene type).
    pub settings: SyncSettings,
    /// The plan of a `planned` scene.
    #[serde(default)]
    pub plan: Option<ScenePlan>,
    /// Objects moved by hand: new ground position (x, z) of their anchor.
    #[serde(default)]
    pub moves: Vec<(ObjectId, [f32; 2])>,
    /// Sensor ground positions.
    #[serde(default)]
    pub sensors: Vec<SceneSensor>,
    /// The camera when it was saved.
    #[serde(default)]
    pub view: Option<CameraView>,
    /// Name of the sensor file (in the project's sensor list).
    #[serde(default)]
    pub sensor_file: Option<String>,
    /// Project item id of the luminaire.
    #[serde(default)]
    pub luminaire: Option<String>,
}
