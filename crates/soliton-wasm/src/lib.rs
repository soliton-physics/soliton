//! WebAssembly surface of the Soliton Physics engine: the wave solvers, the mechanics playground protocol
//! (design 006), the ray optics bench (design 013) and the thermal experiment (design 014).

mod mechanics;
mod optics;
mod thermal;
mod waves;

pub use mechanics::MechanicsWorld;
pub use optics::OpticsBench;
pub use thermal::ThermalGas;
pub use waves::WaveSimulation;
