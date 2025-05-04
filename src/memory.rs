use x86_64::registers::control::{Cr3, Cr3Flags};
use x86_64::structures::paging::{PageTableFlags, PhysFrame};
use x86_64::{PhysAddr};

const PAGE_PRESENT: u64 = 1;
const PAGE_WRITABLE: u64 = 1 << 1;
const PAGE_HUGE: u64 = 1 << 7; // Huge page flag for PDP entries (1GiB)

#[repr(align(4096))]
struct PageTable([u64; 512]);

static mut PML4_TABLE: PageTable = PageTable([0; 512]);
static mut PDP_TABLE: PageTable = PageTable([0; 512]);

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