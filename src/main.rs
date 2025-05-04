#![no_std]
#![no_main]
#![feature(offset_of)]

use core::panic::PanicInfo;
use core::str;

mod font;
mod graphics;
use graphics::{FrameBuffer, COLOR_BLUE, COLOR_WHITE, COLOR_RED, COLOR_GREEN};

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
    
    // BootServices ポインタ (ExitBootServices 前)
    let bs_before = system_table.boot_services as *const _ as usize;

    // BootServices との決別: ExitBootServices を呼び出す
    let mut mmap = MemoryMapHolder::new();
    exit_from_efi_boot_services(image_handle, system_table, &mut mmap);

    // BootServices ポインタ (ExitBootServices 後)
    let bs_after = system_table.boot_services as *const _ as usize;

    // 表示（高さを事前にコピーして借用競合を避ける）
    let h = fb.height;
    draw_hex_message(&mut fb, 10, h - 40, "BS before=0x", bs_before);
    draw_hex_message(&mut fb, 10, h - 20, "BS after =0x", bs_after);

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

/// `label` + 16進表記で数値を描画
fn draw_hex_message(fb: &mut FrameBuffer, x: usize, y: usize, label: &str, value: usize) {
    fb.draw_text(x, y, label, COLOR_WHITE);
    // 16 桁の 0 埋め 16 進文字列を生成
    let mut buf = [0u8; 18]; // "0x" + 16桁
    buf[0] = b'0';
    buf[1] = b'x';
    for i in 0..16 {
        let nibble = ((value >> ((15 - i) * 4)) & 0xF) as u8;
        buf[2 + i] = match nibble {
            0..=9 => b'0' + nibble,
            _ => b'A' + (nibble - 10),
        };
    }
    if let Ok(hex_str) = str::from_utf8(&buf) {
        fb.draw_text(x + label.len() * 10, y, hex_str, COLOR_WHITE);
    }
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
