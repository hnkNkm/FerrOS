// SPDX-License-Identifier: MIT
#![allow(dead_code)]
//! `graphics` モジュール
//! 
//! - ピクセル・矩形・テキスト描画などの基本 API を提供します。
//! - UEFI のフレームバッファへのアクセスを想定しており 32-bit BGRX/RGBX 形式を扱います。

use core::cmp::{max, min};
use crate::font;

/// 24bit カラー定数 (0xRRGGBB)
/// 必要に応じて追加してください。
pub const COLOR_BLACK: u32 = 0x000000;
pub const COLOR_WHITE: u32 = 0xFFFFFF;
pub const COLOR_BLUE: u32 = 0x0000FF;
pub const COLOR_RED: u32 = 0xFF0000;
pub const COLOR_GREEN: u32 = 0x00FF00;
pub const COLOR_YELLOW: u32 = 0xFFFF00;

/// フレームバッファハンドル
pub struct FrameBuffer<'a> {
    /// フレームバッファ(仮想アドレス) 32bit 色値リニア配列
    vram: &'a mut [u32],
    pub width: usize,
    pub height: usize,
}

impl<'a> FrameBuffer<'a> {
    /// 新しい `FrameBuffer` を生成
    pub fn new(vram: &'a mut [u32], width: usize, height: usize) -> Self {
        Self { vram, width, height }
    }

    /// ピクセルを描画 (境界チェック付き)
    pub fn draw_pixel(&mut self, x: usize, y: usize, color: u32) {
        if x < self.width && y < self.height {
            let idx = y * self.width + x;
            self.vram[idx] = color;
        }
    }

    /// 画面全体を単色で塗りつぶし
    pub fn clear(&mut self, color: u32) {
        for px in self.vram.iter_mut() {
            *px = color;
        }
    }

    /// 矩形描画 (塗りつぶし)
    pub fn fill_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: u32) {
        let x_end = min(x.saturating_add(w), self.width);
        let y_end = min(y.saturating_add(h), self.height);

        for yy in y..y_end {
            let row_start = yy * self.width;
            for xx in x..x_end {
                self.vram[row_start + xx] = color;
            }
        }
    }

    /// 矩形の枠線のみ描画
    pub fn stroke_rect(&mut self, x: usize, y: usize, w: usize, h: usize, color: u32) {
        if w == 0 || h == 0 {
            return;
        }
        // 上下ライン
        for xx in x..min(x + w, self.width) {
            self.draw_pixel(xx, y, color);
            self.draw_pixel(xx, y + h.saturating_sub(1), color);
        }
        // 左右ライン
        for yy in y..min(y + h, self.height) {
            self.draw_pixel(x, yy, color);
            self.draw_pixel(x + w.saturating_sub(1), yy, color);
        }
    }

    /// 簡易ライン描画 (Bresenham)
    pub fn draw_line(&mut self, mut x0: isize, mut y0: isize, x1: isize, y1: isize, color: u32) {
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        let width_i = self.width as isize;
        let height_i = self.height as isize;

        loop {
            if x0 >= 0 && y0 >= 0 && x0 < width_i && y0 < height_i {
                self.draw_pixel(x0 as usize, y0 as usize, color);
            }
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = err * 2;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    /// 円の塗りつぶし描画 (Midpoint circle algorithm)
    pub fn fill_circle(&mut self, cx: isize, cy: isize, radius: isize, color: u32) {
        if radius <= 0 {
            return;
        }
        let mut x = radius;
        let mut y = 0;
        let mut err = 0;

        while x >= y {
            self.draw_hline_span(cx, cy, x, y, color);
            y += 1;
            if err <= 0 {
                err += 2 * y + 1;
            } else {
                x -= 1;
                err -= 2 * x + 1;
            }
        }
    }

    /// 文字描画 (8x8 フォント)
    pub fn draw_char(&mut self, x: usize, y: usize, ch: char, color: u32) {
        if let Some(bitmap) = font_for(ch) {
            for (dy, line) in bitmap.iter().enumerate() {
                for dx in 0..8 {
                    if (line >> (7 - dx)) & 0x01 != 0 {
                        self.draw_pixel(x + dx, y + dy, color);
                    }
                }
            }
        }
    }

    /// テキスト描画 (横方向に等幅 8px、1 文字間 2px)
    pub fn draw_text(&mut self, mut x: usize, y: usize, text: &str, color: u32) {
        for ch in text.chars() {
            self.draw_char(x, y, ch, color);
            x += 8 + 2; // 文字幅 + スペース
        }
    }

    /// Midpoint circle 用の水平スパン描画
    fn draw_hline_span(&mut self, cx: isize, cy: isize, x: isize, y: isize, color: u32) {
        let (cx, cy) = (cx as isize, cy as isize);
        let width = self.width as isize;
        let height = self.height as isize;

        let pairs = [
            (cx - x, cx + x, cy + y), // 下側
            (cx - x, cx + x, cy - y), // 上側
            (cx - y, cx + y, cy + x), // 右側
            (cx - y, cx + y, cy - x), // 左側
        ];

        for &(x_start, x_end, yy) in &pairs {
            if yy < 0 || yy >= height {
                continue;
            }
            let xs = max(x_start, 0);
            let xe = min(x_end, width - 1);
            let row = yy as usize * self.width;
            for xx in xs..=xe {
                self.vram[row + xx as usize] = color;
            }
        }
    }

    pub fn draw_hex(&mut self, x: usize, y: usize, val: usize, color: u32) {
        let mut buf = [0u8; 2 + 16]; // "0x" + max 16 hex digits for usize
        buf[0] = b'0';
        buf[1] = b'x';

        let mut current = val;
        // Temporary buffer to store digits in reverse order
        let mut digits = [0u8; 16];
        let mut count = 0;

        if current == 0 {
            digits[0] = b'0';
            count = 1;
        } else {
            // Extract digits in reverse order
            while current > 0 && count < 16 {
                let digit = (current % 16) as u8;
                digits[count] = if digit < 10 {
                    b'0' + digit
                } else {
                    b'A' + digit - 10 // Use uppercase hex
                };
                current /= 16;
                count += 1;
            }
        }

        // Write digits to the main buffer in correct order
        let mut buf_idx = 2; // Start writing after "0x"
        for i in (0..count).rev() {
            buf[buf_idx] = digits[i];
            buf_idx += 1;
        }

        // total_len is the index *after* the last written character,
        // which is also the length of the slice needed.
        let total_len = buf_idx;

        // Safety: We constructed the UTF-8 string correctly using ASCII characters.
        let s = unsafe { core::str::from_utf8_unchecked(&buf[..total_len]) };
        self.draw_text(x, y, s, color);
    }
}

// ======================= フォント取得 ===============================
/// 指定 Unicode 文字の 8x8 ビットマップを返す。
fn font_for(ch: char) -> Option<&'static [u8; 8]> {
    font::glyph(ch)
} 