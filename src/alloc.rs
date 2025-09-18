//! 内存分配器模块
//!
//! 提供高性能内存分配器支持，包括mimalloc等可选分配器

#[cfg(feature = "mimalloc")]
use mimalloc::MiMalloc;

#[cfg(feature = "testing-shred-allocator")]
pub mod testing {
    //! 测试专用的碎片化分配器

    use std::alloc::{GlobalAlloc, Layout};
    use std::sync::atomic::{AtomicUsize, Ordering};

    pub struct ShredAllocator {
        allocated: AtomicUsize,
        freed: AtomicUsize,
    }

    impl ShredAllocator {
        pub const fn new() -> Self {
            Self {
                allocated: AtomicUsize::new(0),
                freed: AtomicUsize::new(0),
            }
        }

        pub fn get_stats(&self) -> (usize, usize) {
            (
                self.allocated.load(Ordering::Relaxed),
                self.freed.load(Ordering::Relaxed),
            )
        }
    }

    unsafe impl GlobalAlloc for ShredAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let size = layout.size();
            self.allocated.fetch_add(size, Ordering::Relaxed);

            // 特意制造碎片化，随机分配不同大小的块
            let padded_size = size + (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as usize % 64);

            std::alloc::System.alloc(layout.pad_to_align())
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            let size = layout.size();
            self.freed.fetch_add(size, Ordering::Relaxed);
            std::alloc::System.dealloc(ptr, layout);
        }
    }
}

#[cfg(feature = "testing-count-allocator")]
pub mod testing_allocator {
    //! 测试专用的计数分配器

    use std::alloc::{GlobalAlloc, Layout};
    use std::sync::atomic::{AtomicUsize, Ordering};

    pub struct CountAllocator {
        total_allocated: AtomicUsize,
        total_freed: AtomicUsize,
        allocation_count: AtomicUsize,
    }

    impl CountAllocator {
        pub const fn new() -> Self {
            Self {
                total_allocated: AtomicUsize::new(0),
                total_freed: AtomicUsize::new(0),
                allocation_count: AtomicUsize::new(0),
            }
        }

        pub fn get_stats(&self) -> (usize, usize, usize) {
            (
                self.total_allocated.load(Ordering::Relaxed),
                self.total_freed.load(Ordering::Relaxed),
                self.allocation_count.load(Ordering::Relaxed),
            )
        }

        pub fn reset_stats(&self) {
            self.total_allocated.store(0, Ordering::Relaxed);
            self.total_freed.store(0, Ordering::Relaxed);
            self.allocation_count.store(0, Ordering::Relaxed);
        }
    }

    unsafe impl GlobalAlloc for CountAllocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let size = layout.size();
            self.total_allocated.fetch_add(size, Ordering::Relaxed);
            self.allocation_count.fetch_add(1, Ordering::Relaxed);

            std::alloc::System.alloc(layout)
        }

        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            let size = layout.size();
            self.total_freed.fetch_add(size, Ordering::Relaxed);
            std::alloc::System.dealloc(ptr, layout);
        }
    }
}

/// 全局内存分配器配置
///
/// 根据启用的特性选择合适的内存分配器
#[cfg(feature = "mimalloc")]
#[global_allocator]
static GLOBAL_ALLOCATOR: MiMalloc = MiMalloc;

#[cfg(all(feature = "testing-shred-allocator", not(feature = "mimalloc")))]
use self::testing::ShredAllocator;

#[cfg(all(feature = "testing-shred-allocator", not(feature = "mimalloc")))]
#[global_allocator]
static GLOBAL_ALLOCATOR: ShredAllocator = ShredAllocator::new();

#[cfg(all(feature = "testing-count-allocator", not(any(feature = "mimalloc", feature = "testing-shred-allocator"))))]
use self::testing_allocator::CountAllocator;

#[cfg(all(feature = "testing-count-allocator", not(any(feature = "mimalloc", feature = "testing-shred-allocator"))))]
#[global_allocator]
static GLOBAL_ALLOCATOR: CountAllocator = CountAllocator::new();


