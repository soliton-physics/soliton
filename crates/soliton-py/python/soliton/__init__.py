"""Research-grade, educational physics engine: spectral wave solvers and planar mechanics."""

from soliton import mechanics
from soliton._native import Simulation, kdv_soliton, nls_bright_soliton

__all__ = ["Simulation", "kdv_soliton", "nls_bright_soliton", "mechanics"]
