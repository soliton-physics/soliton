use num_complex::Complex64;

/// A number a set of linear equations can be solved in: real for steady and stepped circuits, complex for
/// alternating current.
pub trait Scalar:
    Copy
    + std::ops::Add<Output = Self>
    + std::ops::Sub<Output = Self>
    + std::ops::Mul<Output = Self>
    + std::ops::Div<Output = Self>
    + std::ops::AddAssign
    + std::ops::SubAssign
{
    const ZERO: Self;
    fn size(self) -> f64;
}

impl Scalar for f64 {
    const ZERO: Self = 0.0;
    fn size(self) -> f64 {
        self.abs()
    }
}

impl Scalar for Complex64 {
    const ZERO: Self = Complex64::new(0.0, 0.0);
    fn size(self) -> f64 {
        self.norm()
    }
}

/// Solves `a x = b` by elimination, pivoting on the largest entry of each column. `None` when the equations do
/// not determine an answer: a loop of ideal batteries, say.
pub fn solve<T: Scalar>(mut a: Vec<Vec<T>>, mut b: Vec<T>) -> Option<Vec<T>> {
    let n = b.len();
    let scale = a
        .iter()
        .flat_map(|row| row.iter().map(|v| v.size()))
        .fold(0.0, f64::max);
    for column in 0..n {
        let pivot = (column..n).max_by(|&p, &q| {
            a[p][column]
                .size()
                .partial_cmp(&a[q][column].size())
                .unwrap_or(std::cmp::Ordering::Equal)
        })?;
        if a[pivot][column].size() <= 1e-13 * scale.max(1e-300) {
            return None;
        }
        a.swap(column, pivot);
        b.swap(column, pivot);
        for row in column + 1..n {
            let factor = a[row][column] / a[column][column];
            if factor.size() == 0.0 {
                continue;
            }
            let (above, below) = a.split_at_mut(row);
            for (entry, pivot) in below[0][column..].iter_mut().zip(&above[column][column..]) {
                *entry -= factor * *pivot;
            }
            let term = factor * b[column];
            b[row] -= term;
        }
    }
    let mut x = vec![T::ZERO; n];
    for row in (0..n).rev() {
        let mut sum = b[row];
        for k in row + 1..n {
            sum -= a[row][k] * x[k];
        }
        x[row] = sum / a[row][row];
    }
    Some(x)
}
