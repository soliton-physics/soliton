//! Soliton Physics circuits pillar (design 015).
//!
//! A circuit is solved from Kirchhoff's two laws and nothing else: the currents into every junction add to
//! zero, and every component relates the potential across it to the current through it. Series and parallel
//! rules, the bridge condition and the impedance of an LCR circuit are not built in; they are what the solution
//! shows. The same equations are solved three ways: once for steady currents, step by step in time for
//! capacitors and inductors charging, and with complex numbers for alternating current.

mod circuit;
mod linear;
pub mod scenario;
pub mod scene;

pub use circuit::{Circuit, Element, Kind, Phasor, Reading, State};
pub use num_complex::Complex64;
