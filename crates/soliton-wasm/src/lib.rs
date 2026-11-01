//! WebAssembly surface of the Soliton Physics engine: the wave solvers and the mechanics playground protocol
//! (design 006).

mod mechanics;
mod waves;

pub use mechanics::MechanicsWorld;
pub use waves::WaveSimulation;
