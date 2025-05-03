#![no_std]
#![no_main]
#![feature(offset_of)]

use core::mem::offset_of;
use core::mem::size_of;
use core::panic::PanicInfo;
use core::ptr::null_mut;
use core::slice;

type EfiVoid = u8;
type EfiHandle = u64;
type Result<T> = core::result::Result<T, &'static str>;

#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
struct EfiGuid {
    pub data0: u32,
    pub data1: u16,
    pub data2: u16,
    pub data3: [u8; 8],
}

const EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID: EfiGuid = EfiGuid {
    data0: 0x9042a9de,
    data1: 0x23dc,
    data2: 0x4a38,
    data3: [0x96, 0xfb, 0x7a, 0xde, 0xd0, 0x80, 0x51, 0x6a],
};

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
#[must_use]
#[repr(u64)]
enum EfiStatus {
    Success = 0,
}

#[repr(C)]
struct EfiBootServicesTable {
    _reserved0: [u64; 40],
    locate_protocol: extern "win64" fn(
        protocol: *const EfiGuid,
        registration: *const EfiVoid,
        interface: *mut *mut EfiVoid,
    ) -> EfiStatus,
}
const _: () = assert!(offset_of!(EfiBootServicesTable, locate_protocol) == 320);

#[repr(C)]
struct EfiSystemTable {
    _reserved0: [u64; 12],
    pub boot_services: &'static EfiBootServicesTable,
}
const _: () = assert!(offset_of!(EfiSystemTable, boot_services) == 96);

#[repr(C)]
#[derive(Debug)]
struct EfiGraphicsOutputProtocolPixelInfo {
    version: u32,
    pub horizontal_resolution: u32,
    pub vertical_resolution: u32,
    _padding0: [u32; 5],
    pub pixels_per_scan_line: u32,
}
const _: () = assert!(size_of::<EfiGraphicsOutputProtocolPixelInfo>() == 36);

#[repr(C)]
#[derive(Debug)]
struct EfiGraphicsOutputProtocolMode<'a> {
    pub max_mode: u32,
    pub mode: u32,
    pub info: &'a EfiGraphicsOutputProtocolPixelInfo,
    pub size_of_info: u64,
    pub frame_buffer_base: usize,
    pub frame_buffer_size: usize,
}

#[repr(C)]
#[derive(Debug)]
struct EfiGraphicsOutputProtocol<'a> {
    reserved: [u64; 3],
    pub mode: &'a EfiGraphicsOutputProtocolMode<'a>,
}
fn locate_graphic_protocol<'a>(
    efi_system_table: &EfiSystemTable,
) -> Result<&'a EfiGraphicsOutputProtocol<'a>> {
    let mut graphic_output_protocol = null_mut::<EfiGraphicsOutputProtocol>();
    let status = (efi_system_table.boot_services.locate_protocol)(
        &EFI_GRAPHICS_OUTPUT_PROTOCOL_GUID,
        null_mut::<EfiVoid>(),
        &mut graphic_output_protocol as *mut *mut EfiGraphicsOutputProtocol
            as *mut *mut EfiVoid,
    );
    if status != EfiStatus::Success {
        return Err("Failed to locate graphics output protocol");
    }
    Ok(unsafe { &*graphic_output_protocol })
}

// 色定数
const COLOR_BLUE: u32 = 0x0000FF;
const COLOR_WHITE: u32 = 0xFFFFFF;

// 簡易的なピクセル描画関数
fn draw_pixel(vram: &mut [u32], x: usize, y: usize, width: usize, color: u32) {
    if x < width {
        vram[y * width + x] = color;
    }
}

// 文字描画用の簡易フォントデータ（8x8ピクセル）
// 'F', 'E', 'R', ' ', 'O', 'S' のビットマップ
const FONT_F: [u8; 8] = [0xFF, 0x80, 0x80, 0xF0, 0x80, 0x80, 0x80, 0x80];
const FONT_E: [u8; 8] = [0xFF, 0x80, 0x80, 0xF0, 0x80, 0x80, 0x80, 0xFF];
const FONT_R: [u8; 8] = [0xF0, 0x88, 0x88, 0xF0, 0xA0, 0x90, 0x88, 0x84];
const FONT_SPACE: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
const FONT_O: [u8; 8] = [0x78, 0x84, 0x84, 0x84, 0x84, 0x84, 0x84, 0x78];
const FONT_S: [u8; 8] = [0x78, 0x84, 0x80, 0x78, 0x04, 0x04, 0x84, 0x78];

// 文字描画関数
fn draw_char(vram: &mut [u32], x: usize, y: usize, width: usize, font: &[u8; 8], color: u32) {
    for dy in 0..8 {
        let line = font[dy];
        for dx in 0..8 {
            if (line >> (7 - dx)) & 0x01 != 0 {
                draw_pixel(vram, x + dx, y + dy, width, color);
            }
        }
    }
}

// テキスト描画関数
fn draw_text(vram: &mut [u32], x: usize, y: usize, width: usize) {
    // "FERR OS" を描画
    draw_char(vram, x, y, width, &FONT_F, COLOR_WHITE);
    draw_char(vram, x + 10, y, width, &FONT_E, COLOR_WHITE);
    draw_char(vram, x + 20, y, width, &FONT_R, COLOR_WHITE);
    draw_char(vram, x + 30, y, width, &FONT_R, COLOR_WHITE);
    draw_char(vram, x + 40, y, width, &FONT_SPACE, COLOR_WHITE);
    draw_char(vram, x + 50, y, width, &FONT_O, COLOR_WHITE);
    draw_char(vram, x + 60, y, width, &FONT_S, COLOR_WHITE);
}

#[no_mangle]
fn efi_main(_image_handle: EfiHandle, efi_system_table: &EfiSystemTable) {
    let efi_graphics_output_protocol =
        locate_graphic_protocol(efi_system_table).unwrap();
    
    // フレームバッファ情報を取得
    let vram_addr = efi_graphics_output_protocol.mode.frame_buffer_base;
    let vram_byte_size = efi_graphics_output_protocol.mode.frame_buffer_size;
    let width = efi_graphics_output_protocol.mode.info.horizontal_resolution as usize;
    let height = efi_graphics_output_protocol.mode.info.vertical_resolution as usize;
    
    // フレームバッファへのアクセス
    let vram = unsafe {
        slice::from_raw_parts_mut(
            vram_addr as *mut u32,
            vram_byte_size / size_of::<u32>(),
        )
    };
    
    // 画面を青色で塗りつぶす
    for e in vram.iter_mut() {
        *e = COLOR_BLUE;
    }
    
    // 画面中央にテキストを描画
    let text_x = width / 2 - 30;
    let text_y = height / 2 - 4;
    draw_text(vram, text_x, text_y, width);
    
    loop {}
}

#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    loop {}
}
