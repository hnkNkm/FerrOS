use x86_64::registers::control::{Cr3, Cr3Flags};
use x86_64::structures::paging::{PhysFrame};
use x86_64::{PhysAddr, VirtAddr};

const PAGE_PRESENT: u64 = 1;
const PAGE_WRITABLE: u64 = 1 << 1;
const PAGE_HUGE: u64 = 1 << 7;

#[repr(align(4096))]
struct PageTable([u64; 512]);

static mut PML4_TABLE: PageTable = PageTable([0; 512]);
static mut PDP_TABLE: PageTable = PageTable([0; 512]);

/// 簡易アイデンティティマッピング (4GiB) を設定し CR3 を更新
pub unsafe fn init_paging() {
    for i in 0..4 {
        let addr = (i as u64) << 30;
        PDP_TABLE.0[i] = addr | PAGE_PRESENT | PAGE_WRITABLE | PAGE_HUGE;
    }
    PML4_TABLE.0[0] = (&PDP_TABLE as *const _ as u64) | PAGE_PRESENT | PAGE_WRITABLE;

    let pml4_frame = PhysFrame::containing_address(PhysAddr::new(&PML4_TABLE as *const _ as u64));
    Cr3::write(pml4_frame, Cr3Flags::empty());
}

/// 小便利メソッド: 丸め処理など
pub trait VirtAddrExt {
    fn align_down(self, align: u64) -> Self;
}

impl VirtAddrExt for VirtAddr {
    fn align_down(self, align: u64) -> Self {
        let mask = !(align - 1);
        VirtAddr::new(self.as_u64() & mask)
    }
}

/// 旧 API 互換
pub unsafe fn init() {
    init_paging();
} 