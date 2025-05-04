// ---- submodules ----
pub mod mapper;
pub mod allocator;

pub use mapper::{init_paging, VirtAddrExt};
pub use allocator::BitmapFrameAllocator;

use linked_list_allocator::LockedHeap;

// ヒープ領域 (仮): 0x0080_0000 (8MiB) - +100KiB
pub const HEAP_START: usize = 0x0080_0000;
pub const HEAP_SIZE: usize = 100 * 1024; // 100KiB

#[global_allocator]
static GLOBAL_ALLOCATOR: LockedHeap = LockedHeap::empty();

/// ヒープを初期化
pub unsafe fn init_heap() {
    GLOBAL_ALLOCATOR.lock().init(HEAP_START as *mut u8, HEAP_SIZE);
} 