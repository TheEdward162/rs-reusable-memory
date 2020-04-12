//! This module contains sort of manual "specializations" for pushing from iterators.

use super::ReusableMemoryBorrow;

impl<'mem, T> ReusableMemoryBorrow<'mem, T> {
	/// Pushes new values from `iter: impl Iterator` while possible.
	///
	/// Returns the remaining iterator if `self.len()` reaches capacity.
	///
	/// Note that the returned iterator might be exhausted.
	/// Use [`push_from_iter_peeking`](#method.push_from_iter_peeking)
	/// to only return `Err` when `iter` is not exhausted.
	pub fn push_from_iter<I: Iterator<Item = T>>(&mut self, mut iter: I) -> Result<(), I> {
		while self.len() < self.capacity().get() {
			match iter.next() {
				Some(value) => self.push(value).unwrap(),
				None => return Ok(())
			}
		}

		Err(iter)
	}

	/// Pushes new values from `iter: impl Iterator` and peeks ahead.
	///
	/// Returns the remaining iterator (wrapped in `Peekable`)
	/// if `self.len()` reaches capacity and it is not exhausted.
	pub fn push_from_iter_peeking<I: Iterator<Item = T>>(
		&mut self, mut iter: I
	) -> Result<(), std::iter::Peekable<I>> {
		while self.len() < self.capacity().get() {
			match iter.next() {
				Some(value) => self.push(value).unwrap(),
				None => return Ok(())
			}
		}

		let mut iter = iter.peekable();
		if iter.peek().is_none() {
			return Ok(())
		}

		Err(iter)
	}

	/// Pushes new values from `iter: impl ExactSizeIterator`.
	///
	/// Returns the iterator if there is not enough capacity.
	pub fn push_from_exact_iter<I: ExactSizeIterator<Item = T>>(
		&mut self, iter: I
	) -> Result<(), I> {
		if self.len + iter.len() > self.capacity.get() {
			return Err(iter)
		}

		for elem in iter {
			self.push(elem).unwrap()
		}

		Ok(())
	}
}
