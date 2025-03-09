// Copyright (c) 2025 tommyZZM
// tommys-comfy-screen-capturer is licensed under Mulan PSL v2.
// You can use this software according to the terms and conditions of the Mulan PSL v2.
// You may obtain a copy of Mulan PSL v2 at:
//         http://license.coscl.org.cn/MulanPSL2
// THIS SOFTWARE IS PROVIDED ON AN "AS IS" BASIS, WITHOUT WARRANTIES OF ANY KIND,
// EITHER EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO NON-INFRINGEMENT,
// MERCHANTABILITY OR FIT FOR A PARTICULAR PURPOSE.
// See the Mulan PSL v2 for more details.
//

use image::RgbaImage;
use scopeguard::guard;
use std::mem;
use tracing;
use windows::Win32::Foundation::POINT;
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongW, SetLayeredWindowAttributes, SetWindowLongW, GWL_EXSTYLE, LWA_COLORKEY,
    WS_EX_LAYERED,
};
use windows::Win32::{
    Foundation::{GetLastError, HWND},
    Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDIBits,
        GetMonitorInfoW, GetWindowDC, MonitorFromWindow, ReleaseDC, SelectObject, BITMAP,
        BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, HBITMAP, HDC, MONITORINFO,
        MONITOR_DEFAULTTONEAREST, SRCCOPY,
    },
    UI::WindowsAndMessaging::{GetWindowInfo, WINDOWINFO},
};
use windows::Win32::{
    Graphics::Gdi::ClientToScreen,
    UI::WindowsAndMessaging::{GetClientRect, GetDesktopWindow, ShowWindow, SW_HIDE, SW_SHOW},
};
use xcap::{XCapError, XCapResult};

// pub fn get_window_pid(hwnd: HWND) -> u32 {
//     unsafe {
//         let mut lp_dw_process_id = 0;
//         GetWindowThreadProcessId(hwnd, Some(&mut lp_dw_process_id));
//         lp_dw_process_id
//     }
//   }

pub fn bgra_to_rgba(mut buffer: Vec<u8>) -> Vec<u8> {
    // let is_old_version = get_windows_major_version() < 8;
    for src in buffer.chunks_exact_mut(4) {
        src.swap(0, 2);
        // // fix https://github.com/nashaofu/xcap/issues/92#issuecomment-1910014951
        // if src[3] == 0 && is_old_version {
        //     src[3] = 255;
        // }
    }

    buffer
}

pub fn bgra_to_rgba_image(width: u32, height: u32, buffer: Vec<u8>) -> XCapResult<RgbaImage> {
    RgbaImage::from_raw(width, height, bgra_to_rgba(buffer))
        .ok_or_else(|| XCapError::new("RgbaImage::from_raw failed"))
}

fn to_rgba_image(
    hdc_mem: HDC,
    h_bitmap: HBITMAP,
    width: i32,
    height: i32,
) -> XCapResult<RgbaImage> {
    let buffer_size = width * height * 4;
    let mut bitmap_info = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width,
            biHeight: -height,
            biPlanes: 1,
            biBitCount: 32,
            biSizeImage: buffer_size as u32,
            biCompression: 0,
            ..Default::default()
        },
        ..Default::default()
    };

    let mut buffer = vec![0u8; buffer_size as usize];

    unsafe {
        // 读取数据到 buffer 中
        let is_failed = GetDIBits(
            hdc_mem,
            h_bitmap,
            0,
            height as u32,
            Some(buffer.as_mut_ptr().cast()),
            &mut bitmap_info,
            DIB_RGB_COLORS,
        ) == 0;

        if is_failed {
            return Err(XCapError::new("Get RGBA data failed"));
        }
    };

    bgra_to_rgba_image(width as u32, height as u32, buffer)
}

fn delete_bitmap_object(val: HBITMAP) {
    unsafe {
        let succeed = DeleteObject(val.into()).as_bool();

        if !succeed {
            tracing::error!("DeleteObject({:?}) failed: {:?}", val, GetLastError());
        }
    }
}

pub fn capture_screen(target_hwnd: HWND, scale_factor: f32) -> XCapResult<RgbaImage> {
    unsafe {
        // Hide the target window
        ShowWindow(target_hwnd, SW_HIDE);

        // Set the window to be transparent
        let ex_style = GetWindowLongW(target_hwnd, GWL_EXSTYLE);
        SetWindowLongW(target_hwnd, GWL_EXSTYLE, ex_style | WS_EX_LAYERED.0 as i32);
        SetLayeredWindowAttributes(
            target_hwnd,
            windows::Win32::Foundation::COLORREF(0),
            0,
            LWA_COLORKEY,
        );

        let desktop_hwnd = GetDesktopWindow();
        let h_monitor = MonitorFromWindow(target_hwnd, MONITOR_DEFAULTTONEAREST);
        let mut monitor_info = MONITORINFO::default();
        monitor_info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        GetMonitorInfoW(h_monitor, &mut monitor_info);

        let h_monitor = guard(GetWindowDC(Some(desktop_hwnd)), |val| {
            if ReleaseDC(Some(desktop_hwnd), val) != 1 {
                tracing::error!("ReleaseDC({:?}) failed: {:?}", val, GetLastError());
            }
        });

        // 内存中的HDC，使用 DeleteDC 函数释放
        // https://learn.microsoft.com/zh-cn/windows/win32/api/wingdi/nf-wingdi-createcompatibledc
        let hdc_mem = guard(CreateCompatibleDC(Some(*h_monitor)), |val| {
            if !DeleteDC(val).as_bool() {
                tracing::error!("DeleteDC({:?}) failed: {:?}", val, GetLastError());
            }
        });

        let mut rect = mem::zeroed();
        GetClientRect(target_hwnd, &mut rect);

        let mut top_left = POINT {
            x: rect.left,
            y: rect.top,
        };
        ClientToScreen(target_hwnd, &mut top_left);

        let width = ((rect.right - rect.left) as f32 * scale_factor).ceil() as i32;
        let height = ((rect.bottom - rect.top) as f32 * scale_factor).ceil() as i32;

        let h_bitmap = guard(
            CreateCompatibleBitmap(*h_monitor, width, height),
            delete_bitmap_object,
        );
        SelectObject(*hdc_mem, (*h_bitmap).into());

        println!(
            "width: {}, height: {} rect.left: {} rect.top: {}",
            width, height, rect.left, rect.top
        );

        BitBlt(
            *hdc_mem,
            0,
            0,
            width,
            height,
            Some(*h_monitor),
            top_left.x,
            top_left.y,
            SRCCOPY,
        );

        // Restore the window's original style
        SetWindowLongW(target_hwnd, GWL_EXSTYLE, ex_style);
        ShowWindow(target_hwnd, SW_SHOW);

        to_rgba_image(*hdc_mem, *h_bitmap, width, height)
    }
}
