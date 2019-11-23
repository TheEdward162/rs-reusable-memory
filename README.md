# Reusable memory

This Rust crate provides a way to reuse allocated memory for different types.

## Basic usage

To reuse memory, it needs to be allocated first:
```rust
let mut memory: ReusableMemory<u8> = ReusableMemory::new().unwrap();
```
*`new` will return an `Err` if the generic type passed to `ReusableMemory` is a zero sized type.*

The memory can then be borrowed as a different type:
```rust
let mut borrowed_memory = memory.borrow_mut_as::<usize>(NonZeroUsize::new(3).unwrap()).unwrap();
```
*`borrow_mut_as` will return an `Err` if the generic type passed is a zero sized type.*

Now `borrowed_memory` holds a pointer to enough memory to store 3 properly-aligned `usize`s inside the memory allocated in `memory`.
```rust
borrowed_memory.push(1).unwrap();
borrowed_memory.push(2).unwrap();
borrowed_memory.push(std::usize::MAX).unwrap();
```
*`push` will return an `Err` if the pushed value would not fit into the capacity of the borrowed memory.*

The borrowed memory is automatically returned when the object is dropped, and the pushed values are dropped as well.