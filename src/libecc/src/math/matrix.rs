use super::{field::*, vectorized::Vectorized};
use crate::error::*;
use core::{
  fmt::Debug,
  ops::{Add, Div, Mul, Sub},
};

#[derive(Debug, PartialEq, Clone)]
pub struct Matrix<T>(pub Vec<Vectorized<T>>);

impl<T> Mul for Matrix<T>
where
  T: Debug + Add<Output = T> + Mul<Output = T> + Copy + PartialEq,
{
  type Output = Self;

  fn mul(self, right: Self) -> Self::Output {
    assert_eq!(self.col_size(), right.row_size());
    let res: Vec<Vectorized<T>> = self
      .0
      .iter()
      .map(|left_row| {
        let acc = right.0[0].mul_scalar(left_row.0[0]);
        (1..self.col_size()).fold(acc, |acc, idx| {
          acc + right.0[idx].mul_scalar(left_row.0[idx])
        })
      })
      .collect();
    Matrix(res)
  }
}

impl Matrix<GF256> {
  pub fn of_gf256_from_u8(src: &[Vec<u8>]) -> Result<Self> {
    Matrix::new(
      &src
        .iter()
        .map(|v| v.iter().map(|x| GF256(*x)).collect::<Vec<GF256>>())
        .collect::<Vec<Vec<GF256>>>(),
    )
  }

  pub fn mul_on_vec_from_right(&self, coef: &Vectorized<GF256>) -> Vectorized<GF256> {
    let init = Vectorized(vec![GF256(0); self.col_size()]);
    let v = self
      .0
      .iter()
      .enumerate()
      .fold(init, |acc, (i, x)| acc + x.mul_scalar(coef.0[i]));
    v
  }
}

impl<T> Matrix<T>
where
  T: Debug + Clone + PartialEq,
{
  pub fn new(src: &[Vec<T>]) -> Result<Self> {
    ensure!(!src.is_empty(), "null matrix");
    ensure!(
      src.iter().all(|v| v.len() == src[0].len()),
      "Invalid input as a matrix"
    );
    Ok(Matrix(
      src.iter().map(|v| Vectorized(v.to_vec())).collect(),
    ))
  }
  pub fn row_size(&self) -> usize {
    self.0.len()
  }
  pub fn col_size(&self) -> usize {
    self.0[0].len()
  }
  pub fn can_try_diag(&self) -> bool {
    self.row_size() <= self.col_size()
  }
  pub fn is_square(&self) -> bool {
    self.row_size() == self.col_size()
  }

  pub fn is_identity_matrix(&self, zero_t: T, identity_t: T) -> bool {
    if self.col_size() != self.row_size() {
      false
    } else {
      self.0.iter().enumerate().all(|(row_idx, v)| {
        v.0.iter().enumerate().all(|(col_idx, x)| {
          if row_idx == col_idx {
            *x == identity_t
          } else {
            *x == zero_t
          }
        })
      })
    }
  }

  pub fn inverse_left_submatrix(&self, zero_t: T, identity_t: T) -> Result<Self>
  where
    T: Debug + Clone + Copy + PartialEq + Div<Output = T> + Mul<Output = T> + Sub<Output = T>,
  {
    ensure!(self.can_try_diag(), "Invalid matrix size");
    let row_size = self.row_size();

    let expanded = &mut self
      .0
      .iter()
      .enumerate()
      .map(|(row_idx, row)| {
        let mut x = row.clone();
        x.extend_from_slice({
          let mut identity_mat = vec![zero_t; row_size];
          identity_mat[row_idx] = identity_t;
          identity_mat.clone().as_slice()
        });
        x
      })
      .collect::<Vec<_>>();

    let forward = self.forward(expanded, zero_t, identity_t);
    ensure!(forward.is_ok(), "Cannot invert given matrix");

    let backward = self.backward(expanded, zero_t, identity_t);
    ensure!(backward.is_ok(), "Cannot invert given matrix");

    Ok(Matrix(
      expanded
        .iter_mut()
        .map(|row| {
          row
            .clone()
            .subvec(self.col_size(), self.col_size() + self.row_size())
        })
        .collect(),
    ))
  }

  fn backward(&self, target: &mut [Vectorized<T>], zero_t: T, identity_t: T) -> Result<()>
  where
    T: Copy + PartialEq + Div<Output = T> + Mul<Output = T> + Sub<Output = T>,
  {
    ensure!(self.can_try_diag(), "Invalid matrix size");
    let row_size = self.row_size();

    for ptr in (0..row_size).rev() {
      // find focus
      let mut focus: Vectorized<T> = target[ptr].clone();
      for i in (0..ptr).rev() {
        if focus.0[ptr] == zero_t {
          ensure!(i > 0, "Singular matrix");
          // swap focus with one of ptr+1...row_size-1 -th rows
          focus = target[i - 1].clone();
          target[i - 1] = target[ptr].clone();
          target[ptr] = focus.clone();
        }
      }
      // normalize focus
      let coefficient = identity_t / focus.0[ptr];
      focus.mul_scalar_within(coefficient);
      target[ptr] = focus.clone();

      // subtract focus from upper rows
      for i in (0..ptr).rev() {
        let coefficient = target[i].0[ptr];
        target[i].sub_within(focus.clone().mul_scalar(coefficient));
      }
    }

    Ok(())
  }

  fn forward(&self, target: &mut [Vectorized<T>], zero_t: T, identity_t: T) -> Result<()>
  where
    T: Copy + PartialEq + Div<Output = T> + Mul<Output = T> + Sub<Output = T>,
  {
    let row_size = self.row_size();

    for ptr in 0..row_size {
      // find focus
      let mut focus: Vectorized<T> = target[ptr].clone();
      for i in ptr..row_size {
        if focus.0[ptr] == zero_t {
          ensure!(i < row_size - 1, "Singular matrix");
          // swap focus with one of ptr+1...row_size-1 -th rows
          focus = target[i + 1].clone();
          target[i + 1] = target[ptr].clone();
          target[ptr] = focus.clone();
        }
      }
      // normalize focus
      let coefficient = identity_t / focus.0[ptr];
      focus.mul_scalar_within(coefficient);
      target[ptr] = focus.clone();

      // subtract focus from lower rows
      for item in target.iter_mut().take(row_size).skip(ptr + 1) {
        let coefficient = item.0[ptr];
        item.sub_within(focus.clone().mul_scalar(coefficient));
      }
    }
    Ok(())
  }

  pub fn column_submat(&self, from: usize, to: usize) -> Result<Self> {
    ensure!(self.col_size() >= to && from < to, "Invalid parameter");
    let submatrix = Matrix(
      self
        .0
        .iter()
        .map(|row| row.subvec(from, to))
        .collect::<Vec<_>>(),
    );
    Ok(submatrix)
  }
}

#[cfg(test)]
mod tests {
  use super::super::field::GF256;
  use super::*;

  #[test]
  fn new_works() {
    let mat = Matrix::new(&[
      vec![GF256(1), GF256(0), GF256(0), GF256(0)],
      vec![GF256(1), GF256(1), GF256(1), GF256(4)],
      vec![GF256(1), GF256(1), GF256(3), GF256(0)],
      vec![GF256(1), GF256(2), GF256(0), GF256(0)],
    ]);
    assert!(mat.is_ok());
  }

  #[test]
  fn get_inverse_works() {
    let mat = Matrix::new(&[
      vec![GF256(1), GF256(0), GF256(0), GF256(0)],
      vec![GF256(1), GF256(1), GF256(1), GF256(4)],
      vec![GF256(1), GF256(1), GF256(3), GF256(0)],
      vec![GF256(1), GF256(2), GF256(0), GF256(0)],
    ]);
    assert!(mat.is_ok());
    let matrix = mat.unwrap();
    let inv = matrix.inverse_left_submatrix(GF256(0), GF256(1));
    assert!(inv.is_ok());
    let inverse = inv.unwrap();
    // println!("{:?}", inverse.clone());

    let mult = inverse * matrix;
    // println!("{:?}", mult);
    assert!(mult.is_identity_matrix(GF256(0), GF256(1)));
  }
}
