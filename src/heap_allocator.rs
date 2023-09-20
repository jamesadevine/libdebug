pub use libc::{c_void, size_t};
use std::fmt::{Debug, Formatter, Result};

struct HeapAllocation {
    ptr: *mut c_void,
    real_size: size_t,
    alloc_size: size_t,
}

impl HeapAllocation {
    /// Returns the number of bytes between HeapAllocations
    pub fn distance_to(&self, other: &HeapAllocation) -> i32 {
        other.start() as i32 - self.end() as i32
    }

    pub fn start(&self) -> usize {
        self.ptr as usize
    }

    pub fn end(&self) -> usize {
        return self.start() + self.alloc_size;
    }
}

impl Debug for HeapAllocation {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "HeapAllocation {{ ptr: {:?}, real_size: {}, alloc_size: {} }}",
            self.ptr, self.real_size, self.alloc_size
        )
    }
}

pub struct Heap {
    ptr: *mut c_void,
    size: size_t,
    allocations: Vec<HeapAllocation>,
}

impl Heap {
    pub fn new(size: size_t) -> Heap {
        let ptr = unsafe { libc::malloc(size) };
        return Heap {
            ptr,
            size,
            allocations: vec![],
        };
    }

    fn end(&self) -> usize {
        return self.ptr as usize + self.size;
    }

    fn align(size: size_t) -> size_t {
        return size + (size % 4);
    }

    fn used_space(&self) -> usize {
        let mut space = 0;
        for allocation in self.allocations.iter() {
            space += allocation.alloc_size;
        }
        return space;
    }
    fn free_space(&self) -> usize {
        return self.size - self.used_space();
    }

    fn next_for_size(&self, size: usize) -> (*mut c_void, usize) {
        let mut ptr = self.ptr;
        let mut previous: Option<&HeapAllocation> = None;

        for (index, allocation) in self.allocations.iter().enumerate() {
            if let Some(prev) = previous {
                println!("{} {}", prev.distance_to(allocation), size);
                if prev.distance_to(allocation) >= size as i32 {
                    return (prev.end() as *mut c_void, index);
                }
            }
            previous = Some(allocation);
            ptr = unsafe { ptr.add(allocation.alloc_size) };
        }
        return (ptr, self.allocations.len());
    }

    fn allocate(&mut self, size: size_t) -> *mut c_void {
        let alloc_size = Self::align(size);
        let (ptr, index) = self.next_for_size(alloc_size);

        if ptr as usize + alloc_size > self.end() {
            return 0 as *mut c_void;
        }
        // TODO: Use log debug/info instead of println!
        println!("allocating: {:?} {} bytes", ptr, alloc_size);
        self.allocations.insert(
            index,
            HeapAllocation {
                ptr,
                real_size: size,
                alloc_size: alloc_size,
            },
        );

        return ptr;
    }

    fn free(&mut self, ptr: *mut c_void) {
        let mut index = None;
        for (idx, allocation) in self.allocations.iter().enumerate() {
            if allocation.ptr == ptr {
                index = Some(idx);
                break;
            }

            // allocations are contiguous, skip if we exceed the target ptr address
            if allocation.ptr > ptr {
                break;
            }
        }
        if let Some(idx) = index {
            let alloc = self.allocations.remove(idx);
            // TODO: Use log debug/info instead of println!
            println!("freeing: {:?} {:?}", alloc.ptr, alloc.alloc_size);
        }
    }
}

impl Debug for Heap {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "Heap {{ ptr: {:?}, size: {}, used: {}, free: {} alloc_count: {} }}",
            self.ptr,
            self.size,
            self.used_space(),
            self.free_space(),
            self.allocations.len()
        )?;
        for allocation in self.allocations.iter() {
            write!(f, "\n\t{:?}", allocation)?;
        }
        Ok(())
    }
}

pub struct HeapAllocator {
    pub heaps: Vec<Heap>,
}

impl HeapAllocator {
    pub fn init_heap(&mut self, size: usize) {
        self.heaps.push(Heap::new(size));
    }
    pub fn allocate(&mut self, size: size_t) -> *mut c_void {
        if self.heaps.len() == 0 {
            self.init_heap(32768);
        }
        return self.heaps.first_mut().unwrap().allocate(size);
    }

    pub fn free(&mut self, ptr: *mut c_void) {
        if self.heaps.len() == 0 {
            self.init_heap(32768);
        }
        return self.heaps.first_mut().unwrap().free(ptr);
    }
}

impl Debug for HeapAllocator {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "HeapAllocator {{ heap_count: {}}}", self.heaps.len())?;
        for heap in self.heaps.iter() {
            write!(f, "\n\t{:?}", heap)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ensure_heap_contiguity(heap: &Heap) {
        let mut mut_ptr = heap.ptr;
        for alloc in heap.allocations.iter() {
            assert!(mut_ptr == alloc.ptr);
            mut_ptr = unsafe { mut_ptr.add(alloc.alloc_size) };
        }
    }

    #[test]
    fn allocate_all() {
        let mut heap = HeapAllocator { heaps: vec![] };
        for _ in 0..16 {
            assert!(!heap.allocate(2048).is_null());
        }
    }

    #[test]
    fn allocate_too_many() {
        let mut heap = HeapAllocator { heaps: vec![] };
        for _ in 0..16 {
            assert!(!heap.allocate(2048).is_null());
        }
        assert!(heap.allocate(2048).is_null());
    }

    #[test]
    fn ensure_allocations_are_contiguous() {
        let mut heap = HeapAllocator { heaps: vec![] };
        for _ in 0..16 {
            assert!(!heap.allocate(2048).is_null());
        }
        ensure_heap_contiguity(heap.heaps.first().unwrap());
    }

    #[test]
    fn allocate_then_free() {
        let mut heap = HeapAllocator { heaps: vec![] };
        let mut tracked_alloc = None;
        for i in 0..16 {
            let alloc = heap.allocate(2048);
            assert!(!alloc.is_null());
            if i == 8 {
                tracked_alloc = Some(alloc);
            }
        }

        assert!(tracked_alloc.is_some());
        heap.free(tracked_alloc.unwrap());
        assert!(heap.heaps.first().unwrap().allocations.len() == 15);
    }

    #[test]
    fn allocate_then_free_then_allocate() {
        let mut heap = HeapAllocator { heaps: vec![] };
        let mut tracked_alloc = None;
        for i in 0..16 {
            let alloc = heap.allocate(2048);
            assert!(!alloc.is_null());
            if i == 8 {
                tracked_alloc = Some(alloc);
            }
        }
        assert!(tracked_alloc.is_some());
        heap.free(tracked_alloc.unwrap());
        let alloc = heap.allocate(2048);
        assert!(alloc == tracked_alloc.unwrap());
        ensure_heap_contiguity(heap.heaps.first().unwrap());
    }
}
