use x86_64::registers::control::{Cr3, Cr3Flags};
use x86_64::structures::paging::{PhysFrame, FrameAllocator, Size4KiB};
use x86_64::PhysAddr;
use linked_list_allocator::LockedHeap;
use crate::efi::EfiMemoryDescriptor;

const PAGE_PRESENT: u64 = 1;
const PAGE_WRITABLE: u64 = 1 << 1;
const PAGE_HUGE: u64 = 1 << 7; // Huge page flag for PDP entries (1GiB)

#[repr(align(4096))]
struct PageTable([u64; 512]);

static mut PML4_TABLE: PageTable = PageTable([0; 512]);
static mut PDP_TABLE: PageTable = PageTable([0; 512]);

// ヒープ領域 (仮): 0x0080_0000 (8MiB) - +100KiB
pub const HEAP_START: usize = 0x0080_0000;
pub const HEAP_SIZE: usize = 100 * 1024; // 100KiB

#[global_allocator]
static GLOBAL_ALLOCATOR: LockedHeap = LockedHeap::empty();

/// シンプルな恒久アイデンティティマッピング (最大 4GiB) を構築し、CR3 を更新
///
/// Safety: 実行中のコードとデータが 4GiB 以内かつアイデンティティマッピングで
/// アクセス可能であることを前提とする。
pub unsafe fn init() {
    // PDP: 4GiB を 1GiB huge page ×4 で Identity Map
    for i in 0..4 {
        let addr = (i as u64) << 30; // 1GiB 単位
        PDP_TABLE.0[i] = addr | PAGE_PRESENT | PAGE_WRITABLE | PAGE_HUGE;
    }

    // PML4[0] -> PDP テーブル
    PML4_TABLE.0[0] = (&PDP_TABLE as *const _ as u64) | PAGE_PRESENT | PAGE_WRITABLE;

    // CR3 へロード
    let pml4_frame = PhysFrame::containing_address(PhysAddr::new(&PML4_TABLE as *const _ as u64));
    Cr3::write(pml4_frame, Cr3Flags::empty());
}

/// ヒープを初期化
///
/// Safety: `memory::init()` によりページテーブルが生成済みであり、
/// HEAP_START..HEAP_START+HEAP_SIZE 領域がマッピングされている必要がある。
pub unsafe fn init_heap() {
    GLOBAL_ALLOCATOR.lock().init(HEAP_START as *mut u8, HEAP_SIZE);
}

pub struct BootInfoFrameAllocator {
    memory_map: &'static [EfiMemoryDescriptor],
    next_desc: usize,
    offset: u64,
}

impl BootInfoFrameAllocator {
    /// Safety: `holder` のライフタイムが 'static であり、UEFI から取得した
    /// メモリマップバッファであることを前提とする。
    pub unsafe fn new(holder: &crate::efi::MemoryMapHolder) -> Self {
        let num_desc = holder.memory_map_size / holder.descriptor_size;
        let descriptors = core::slice::from_raw_parts(
            holder.memory_map_buffer.as_ptr() as *const EfiMemoryDescriptor,
            num_desc,
        );
        Self {
            memory_map: descriptors,
            next_desc: 0,
            offset: 0,
        }
    }
}

// EfiConventionalMemory = 7
const EFI_CONVENTIONAL: u32 = 7;

unsafe impl FrameAllocator<Size4KiB> for BootInfoFrameAllocator {
    fn allocate_frame(&mut self) -> Option<PhysFrame> {
        loop {
            let desc = self.memory_map.get(self.next_desc)?;
            if desc.memory_type != EFI_CONVENTIONAL {
                self.next_desc += 1;
                self.offset = 0;
                continue;
            }
            let region_start = desc.physical_start;
            let region_size = desc.number_of_pages * 4096;
            let candidate = region_start + self.offset;
            if candidate + 4096 <= region_start + region_size {
                let frame = PhysFrame::containing_address(PhysAddr::new(candidate));
                self.offset += 4096;
                return Some(frame);
            } else {
                self.next_desc += 1;
                self.offset = 0;
                continue;
            }
        }
    }
} 