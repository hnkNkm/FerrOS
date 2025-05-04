use x86_64::structures::idt::{InterruptDescriptorTable, InterruptStackFrame};
use lazy_static::lazy_static;
use crate::graphics::{self, FrameBuffer, COLOR_YELLOW}; // 仮。fb 取得方法が必要

lazy_static! {
    static ref IDT: InterruptDescriptorTable = {
        let mut idt = InterruptDescriptorTable::new();
        unsafe {
            idt.double_fault
                .set_handler_fn(double_fault_handler)
                .set_stack_index(0);
        }
        idt
    };
}

/// IDT をロード
pub fn init() {
    IDT.load();
}

extern "x86-interrupt" fn double_fault_handler(
    _stack_frame: InterruptStackFrame,
    _error_code: u64,
) -> ! {
    // ダブルフォルト発生時に画面を黄色にする (デバッグ用)
    // Safety: フレームバッファアドレスが既知でアクセス可能と仮定
    // 本来は main から安全に渡すか、static などで共有する必要あり
    // const FB_ADDR: usize = 0x????????????; // GOP から取得したアドレス
    // unsafe {
    //    if FB_ADDR != 0 {
    //        (*(FB_ADDR as *mut FrameBuffer)).clear(COLOR_YELLOW);
    //    }
    // }
    loop {}
} 