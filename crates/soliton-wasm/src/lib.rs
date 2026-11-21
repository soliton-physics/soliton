//! WebAssembly surface of the Soliton Physics engine: the wave solvers, the mechanics playground protocol
//! (design 006), the ray optics bench (design 013), the thermal experiment (design 014), circuits (design 015) and statistical experiments (design 016).

mod circuits;
mod matter;
mod mechanics;
mod modern;
mod optics;
mod statmech;
mod thermal;
mod waves;

pub use circuits::CircuitBench;
pub use matter::MatterRun;
pub use mechanics::MechanicsWorld;
pub use modern::ModernRun;
pub use optics::OpticsBench;
pub use statmech::StatRun;
pub use thermal::ThermalGas;
pub use waves::WaveSimulation;
