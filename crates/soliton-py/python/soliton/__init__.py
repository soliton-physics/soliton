"""Research-grade spectral solvers for nonlinear wave equations."""

from soliton._native import Simulation, kdv_soliton, nls_bright_soliton

__all__ = ["Simulation", "kdv_soliton", "nls_bright_soliton"]
