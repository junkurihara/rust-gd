use crate::types::U8VRep;

use super::field::GF256;
use core::ops::{Add, Mul, Sub};

#[derive(Debug, PartialEq, Clone)]
pub struct Vectorized<T>(pub Vec<T>);

impl Vectorized<GF256> {
  pub fn of_gf256_from_u8(slice: &[u8]) -> Self {
    let v: Vec<GF256> = slice.iter().map(|x| GF256(*x)).collect();
    Vectorized(v)
  }

  pub fn to_u8_vec(&self) -> U8VRep {
    self.0.iter().map(|x| x.0).collect()
  }
}

impl<T> Vectorized<T> {
  pub fn extend_from_slice(&mut self, slice: &[T])
  where
    T: Clone,
  {
    self.0.extend_from_slice(slice)
  }
}

impl<T> Vectorized<T>
where
  T: Clone,
{
  pub fn len(&self) -> usize {
    self.0.len()
  }
  pub fn subvec(&self, start: usize, end: usize) -> Self {
    assert!(end <= self.len() && start <= end, "Invalid params");
    Vectorized((&self.0[start..end]).to_vec())
  }
}

impl<T> Vectorized<T>
where
  T: Mul<Output = T> + Copy,
{
  pub fn mul_scalar(&self, coefficient: T) -> Vectorized<T> {
    Self(self.0.iter().map(|c| c.clone() * coefficient).collect())
  }
}

impl<T> Vectorized<T>
where
  T: Mul<Output = T> + Copy,
{
  pub fn mul_scalar_within(&mut self, coefficient: T) {
    for idx in 0..self.0.len() {
      self.0[idx] = coefficient * self.0[idx];
    }
  }
}

impl<T> Vectorized<T>
where
  T: Add<Output = T> + Copy,
{
  #[allow(dead_code)]
  pub fn add_within(&mut self, other: Self) {
    assert_eq!(self.0.len(), other.0.len());
    for idx in 0..self.0.len() {
      self.0[idx] = self.0[idx] + other.0[idx];
    }
  }
}

impl<T> Vectorized<T>
where
  T: Sub<Output = T> + Copy,
{
  pub fn sub_within(&mut self, other: Self) {
    assert_eq!(self.0.len(), other.0.len());
    for idx in 0..self.0.len() {
      self.0[idx] = self.0[idx] - other.0[idx];
    }
  }
}

impl<T> Add for Vectorized<T>
where
  T: Add<Output = T> + Copy,
{
  type Output = Vectorized<T>;

  fn add(self, other: Self) -> Self::Output {
    assert_eq!(self.0.len(), other.0.len());
    let x: Vec<T> = self
      .0
      .iter()
      .zip(other.0.iter())
      .map(|(x, y)| *x + *y)
      .collect();
    Self(x)
  }
}

impl<T> Sub for Vectorized<T>
where
  T: Sub<Output = T> + Copy,
{
  type Output = Vectorized<T>;

  fn sub(self, other: Self) -> Self::Output {
    assert_eq!(self.0.len(), other.0.len());
    let x: Vec<T> = self
      .0
      .iter()
      .zip(other.0.iter())
      .map(|(x, y)| *x - *y)
      .collect();
    Self(x)
  }
}

impl<T> Mul for Vectorized<T>
where
  T: Add<Output = T> + Mul<Output = T> + Copy,
{
  type Output = T;

  fn mul(self, other: Self) -> Self::Output {
    assert_eq!(self.0.len(), other.0.len());
    assert!(self.0.len() > 0);
    let acc = self.0[0] * other.0[0];
    (1..self.0.len()).fold(acc, |acc, idx| acc + self.0[idx] * other.0[idx])
  }
}

#[cfg(test)]
mod tests {
  use super::super::field::GF256;
  use super::*;

  #[test]
  fn mul_scalar_works() {
    let left = Vectorized(vec![GF256(1), GF256(2), GF256(3), GF256(4)]);
    let coef = GF256(2);
    let right = left.mul_scalar(coef);
    assert_eq!(
      right,
      Vectorized(vec![GF256(2), GF256(4), GF256(6), GF256(8)])
    );
  }
  #[test]
  fn mul_scalar_within_works() {
    let mut left = Vectorized(vec![GF256(1), GF256(2), GF256(3), GF256(4)]);
    let coef = GF256(2);
    left.mul_scalar_within(coef);
    assert_eq!(
      left,
      Vectorized(vec![GF256(2), GF256(4), GF256(6), GF256(8)])
    );
  }
  #[test]
  fn add_within_works() {
    let mut left = Vectorized(vec![GF256(1), GF256(1), GF256(1), GF256(1)]);
    let right = Vectorized(vec![GF256(2), GF256(3), GF256(4), GF256(5)]);
    left.add_within(right);
    assert_eq!(
      left,
      Vectorized(vec![GF256(3), GF256(2), GF256(5), GF256(4)])
    );
  }

  #[test]
  fn sub_within_works() {
    let mut left = Vectorized(vec![GF256(3), GF256(2), GF256(5), GF256(4)]);
    let right = Vectorized(vec![GF256(2), GF256(3), GF256(4), GF256(5)]);
    left.sub_within(right);
    assert_eq!(
      left,
      Vectorized(vec![GF256(1), GF256(1), GF256(1), GF256(1)])
    );
  }

  #[test]
  fn add_works() {
    let left = Vectorized(vec![GF256(1), GF256(1), GF256(1), GF256(1)]);
    let right = Vectorized(vec![GF256(2), GF256(3), GF256(4), GF256(5)]);
    assert_eq!(
      left + right,
      Vectorized(vec![GF256(3), GF256(2), GF256(5), GF256(4)])
    );
  }

  #[test]
  fn sub_works() {
    let left = Vectorized(vec![GF256(1), GF256(1), GF256(1), GF256(1)]);
    let right = Vectorized(vec![GF256(3), GF256(2), GF256(5), GF256(4)]);
    assert_eq!(
      left - right,
      Vectorized(vec![GF256(2), GF256(3), GF256(4), GF256(5)])
    );
  }

  #[test]
  fn mul_works() {
    let left = Vectorized(vec![GF256(1), GF256(1), GF256(1), GF256(1)]);
    let right = Vectorized(vec![GF256(3), GF256(2), GF256(5), GF256(4)]);
    assert_eq!(left * right, GF256(0));
  }
}
