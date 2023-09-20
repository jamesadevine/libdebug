use heap_allocator::HeapAllocator;
pub use libc::{c_void, size_t};
mod heap_allocator;

static mut ALLOCATOR: HeapAllocator = HeapAllocator { heaps: vec![] };

#[no_mangle]
pub extern "C" fn heap_init(size: size_t) {
    unsafe { ALLOCATOR.init_heap(size) }
}

#[no_mangle]
pub extern "C" fn heap_malloc(size: size_t) -> *mut c_void {
    unsafe { ALLOCATOR.allocate(size) }
}

#[no_mangle]
pub extern "C" fn heap_free(ptr: *mut c_void) {
    unsafe { ALLOCATOR.free(ptr) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dummy_allocate() {
        for _ in 0..16 {
            assert!(!heap_malloc(2048).is_null());
        }
        println!("allocator {:?}", unsafe { &ALLOCATOR });
        println!("final malloc {:?}", heap_malloc(2048) as usize);
        assert!(heap_malloc(2048).is_null());
    }
}
