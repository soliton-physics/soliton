//! WebAssembly surface of the Soliton Physics engine: the wave solvers, the mechanics playground protocol
//! (design 006), the ray optics bench (design 013), the thermal experiment (design 014), circuits (design 015) and statistical experiments (design 016).

mod circuits;
mod mechanics;
mod optics;
mod statmech;
mod thermal;
mod waves;

pub use circuits::CircuitBench;
pub use mechanics::MechanicsWorld;
pub use optics::OpticsBench;
pub use statmech::StatRun;
pub use thermal::ThermalGas;
pub use waves::WaveSimulation;
