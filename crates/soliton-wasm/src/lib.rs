//! WebAssembly surface of the Soliton Physics engine: the wave solvers, the mechanics playground protocol
//! (design 006) and the ray optics bench (design 013).

mod mechanics;
mod optics;
mod waves;

pub use mechanics::MechanicsWorld;
pub use optics::OpticsBench;
pub use waves::WaveSimulation;
