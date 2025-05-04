#![no_std]
#![no_main]
#![feature(offset_of)]

use core::panic::PanicInfo;

mod font;
mod graphics;
use graphics::{FrameBuffer, COLOR_WHITE, COLOR_RED, COLOR_GREEN};

mod efi;
use efi::{EfiHandle, EfiSystemTable, framebuffer, MemoryMapHolder, EfiStatus};

// ------------------------------------------------------------
// 簡易 UI モジュール（暫定）
mod ui {
    use crate::graphics::{FrameBuffer, COLOR_BLUE, COLOR_WHITE, COLOR_RED, COLOR_GREEN};

    /// ホーム画面を描画
    pub fn home(fb: &mut FrameBuffer) {
        // 背景
        fb.clear(COLOR_BLUE);

        // テキスト
        let label = "Hello, World!";
        let text_width = label.len() * 8 + (label.len() - 1) * 2;
        let x = (fb.width - text_width) / 2;
        let y = fb.height / 2 - 4;
        fb.draw_text(x, y, label, COLOR_WHITE);

        // 図形例 (円・矩形など)
        fb.fill_rect(20, 20, 80, 50, COLOR_RED);
        fb.stroke_rect(fb.width - 140, 30, 120, 80, COLOR_GREEN);
        fb.fill_circle((fb.width / 2) as isize, (fb.height * 3 / 4) as isize, 40, COLOR_GREEN);
    }
}
// ------------------------------------------------------------

#[no_mangle]
fn efi_main(image_handle: EfiHandle, system_table: &EfiSystemTable) {
    let mut fb = framebuffer(system_table).expect("GOP unavailable");

    // ホーム画面を描画
    ui::home(&mut fb);
    
    // 1 秒待機（1,000,000 マイクロ秒）
    let _ = (system_table.boot_services.stall)(1_000_000usize);

    // BootServices ポインタ (ExitBootServices 前)
    let _bs_before = system_table.boot_services as *const _ as usize;

    // BootServices との決別: ExitBootServices を呼び出す
    let mut mmap = MemoryMapHolder::new();
    exit_from_efi_boot_services(image_handle, system_table, &mut mmap);

    // 以降は Non-UEFI 世界。画面をクリアしてメッセージ表示
    fb.clear(COLOR_RED);
    let label = "Hello, NonUEFI!";
    let text_width = label.len() * 8 + (label.len() - 1) * 2;
    let x = (fb.width - text_width) / 2;
    let y = fb.height / 2 - 4;
    fb.draw_text(x, y, label, COLOR_WHITE);

    loop {}
}

/// ExitBootServices を安全に呼び出すヘルパ
fn exit_from_efi_boot_services(
    image_handle: EfiHandle,
    system_table: &EfiSystemTable,
    map_holder: &mut MemoryMapHolder,
) {
    loop {
        let st = system_table.boot_services;
        let status = st.call_get_memory_map(map_holder);
        assert_eq!(status, EfiStatus::Success);

        let status = (st.exit_boot_services)(image_handle, map_holder.map_key);
        if status == EfiStatus::Success {
            break;
        }
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
