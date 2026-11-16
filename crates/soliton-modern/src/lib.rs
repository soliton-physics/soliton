//! Soliton Physics modern physics pillar (design 017).
//!
//! The physics of the early twentieth century, where light comes in quanta, particles have wavelengths, and
//! atoms and nuclei have structure. Where there is something to follow it is followed: alpha particles are
//! integrated past a nucleus, nuclei decay one by one by chance, photoelectrons are counted as they arrive.
//! The rest is the formulas of Einstein, de Broglie and Bohr, evaluated.

pub mod atom;
pub mod constants;
pub mod frame;
pub mod matter;
pub mod nucleus;
pub mod photoelectric;
pub mod rutherford;
pub mod scenario;

pub use frame::{Answer, Curve, Lesson, Picture, Plot, Reading, Style};
pub use scenario::{catalogue, find, Params, Run};
