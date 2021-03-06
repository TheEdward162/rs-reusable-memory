# Reusable memory

This Rust crate provides a way to reuse allocated memory for different types.

## Basic usage

```rust
use reusable_memory::ReusableMemory;

let mut memory: ReusableMemory<u8> = ReusableMemory::new();

{
	// The memory can then be borrowed as a different type:
	let mut borrowed_memory = memory.borrow_mut_as::<usize>(std::num::NonZeroUsize::new(3).unwrap());

	// Now `borrowed_memory` holds a pointer to enough memory to store 3 properly-aligned `usize`s inside the memory allocated in `memory`.
	borrowed_memory.push(1).unwrap();
	borrowed_memory.push(2).unwrap();
	borrowed_memory.push(std::usize::MAX).unwrap();
	// `push` will return an `Err` if the pushed value would not fit into the capacity of the borrowed memory.

	assert_eq!(borrowed_memory.as_slice(), &[1, 2, std::usize::MAX]);

	// values can also be `pop`ed or `drain`ed as with `Vec`:
	assert_eq!(borrowed_memory.pop(), Some(std::usize::MAX));
	assert_eq!(borrowed_memory.drain(..).collect::<Vec<usize>>().as_slice(), &[1, 2]);
	assert_eq!(borrowed_memory.pop(), None);
}
// The borrowed memory is automatically returned when the object is dropped, and the pushed values are dropped as well.

// Now the memory can be reused, even as multiple different types (current limit is 5 because the code is generated by a macro):
{
	let (mut borrow_t, mut borrow_u) = memory.borrow_mut_two_as::<usize, u8>(
		[
			std::num::NonZeroUsize::new(1).unwrap(),
			std::num::NonZeroUsize::new(2).unwrap()
		]
	);

	borrow_t.push(0usize).unwrap();
	
	borrow_u.push(1u8).unwrap();
	borrow_u.push(2u8).unwrap();

	assert_eq!(borrow_t.as_slice(), &[0usize]);

	assert_eq!(borrow_u.as_slice(), &[1u8, 2u8]);
}
```
