use x86_64::structures::gdt::{Descriptor, GlobalDescriptorTable};
use x86_64::structures::tss::TaskStateSegment;
use x86_64::VirtAddr;
use lazy_static::lazy_static;

// TSS 用の固定スタック
static mut DOUBLE_FAULT_STACK: [u8; 4096] = [0; 4096];

// セグメントセレクタ保持
pub struct Selectors {
    pub code_selector: x86_64::structures::gdt::SegmentSelector,
    pub tss_selector: x86_64::structures::gdt::SegmentSelector,
}

lazy_static! {
    static ref TSS: TaskStateSegment = {
        let mut tss = TaskStateSegment::new();
        // ダブルフォルト用スタック
        let stack_start = unsafe { &DOUBLE_FAULT_STACK as *const _ as u64 };
        let stack_end = stack_start + 4096;
        tss.interrupt_stack_table[0] = VirtAddr::new(stack_end);
        tss
    };

    static ref GDT: (GlobalDescriptorTable, Selectors) = {
        let mut gdt = GlobalDescriptorTable::new();
        let code_selector = gdt.add_entry(Descriptor::kernel_code_segment());
        let tss_selector = gdt.add_entry(Descriptor::tss_segment(&TSS));
        (gdt, Selectors { code_selector, tss_selector })
    };
}

/// GDT と TSS をロード
pub fn init() {
    use x86_64::instructions::segmentation::set_cs;
    use x86_64::instructions::tables::load_tss;

    GDT.0.load();
    unsafe {
        set_cs(GDT.1.code_selector);
        load_tss(GDT.1.tss_selector);
    }
} 