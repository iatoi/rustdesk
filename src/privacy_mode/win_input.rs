use hbb_common::{allow_err, bail, lazy_static, log, ResultType};
use std::{
    io::Error,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc::{channel, Sender},
        Mutex,
    },
    time::Instant,
};
use winapi::{
    ctypes::c_int,
    shared::{
        minwindef::{DWORD, FALSE, HMODULE, LOBYTE, LPARAM, LRESULT, UINT, WPARAM},
        ntdef::NULL,
        windef::{HHOOK, POINT},
    },
    um::{libloaderapi::GetModuleHandleExA, processthreadsapi::GetCurrentThreadId, winuser::*},
};

const GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT: u32 = 2;
const GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS: u32 = 4;

const WM_USER_EXIT_HOOK: u32 = WM_USER + 1;

use super::win_input_ime as ime;

lazy_static::lazy_static! {
    static ref CUR_HOOK_THREAD_ID: Mutex<DWORD> = Mutex::new(0);
    // iatoi-rc: moc thoi gian de do khoang cach tu phim tu xa gan nhat.
    static ref MOC_THOI_GIAN: Instant = Instant::now();
}

// iatoi-rc: ms (tinh tu MOC_THOI_GIAN, +1) cua phim tu xa gan nhat; 0 = chua co.
static PHIM_TU_XA_GAN_NHAT_MS: AtomicU64 = AtomicU64::new(0);

fn bay_gio_ms() -> u64 {
    (MOC_THOI_GIAN.elapsed().as_millis() as u64).saturating_add(1)
}

fn do_hook(tx: Sender<String>) -> ResultType<(HHOOK, HHOOK)> {
    let invalid_ret = (0 as HHOOK, 0 as HHOOK);

    let mut cur_hook_thread_id = CUR_HOOK_THREAD_ID.lock().unwrap();
    if *cur_hook_thread_id != 0 {
        // unreachable!
        tx.send("Already hooked".to_owned())?;
        return Ok(invalid_ret);
    }

    unsafe {
        let mut hm_keyboard = 0 as HMODULE;
        if 0 == GetModuleHandleExA(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            DefWindowProcA as _,
            &mut hm_keyboard as _,
        ) {
            tx.send(format!(
                "Failed to GetModuleHandleExA, error: {}",
                Error::last_os_error()
            ))?;
            return Ok(invalid_ret);
        }
        let mut hm_mouse = 0 as HMODULE;
        if 0 == GetModuleHandleExA(
            GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT,
            DefWindowProcA as _,
            &mut hm_mouse as _,
        ) {
            tx.send(format!(
                "Failed to GetModuleHandleExA, error: {}",
                Error::last_os_error()
            ))?;
            return Ok(invalid_ret);
        }

        let hook_keyboard = SetWindowsHookExA(
            WH_KEYBOARD_LL,
            Some(privacy_mode_hook_keyboard),
            hm_keyboard,
            0,
        );
        if hook_keyboard.is_null() {
            tx.send(format!(
                " SetWindowsHookExA keyboard, error {}",
                Error::last_os_error()
            ))?;
            return Ok(invalid_ret);
        }

        let hook_mouse = SetWindowsHookExA(WH_MOUSE_LL, Some(privacy_mode_hook_mouse), hm_mouse, 0);
        if hook_mouse.is_null() {
            if FALSE == UnhookWindowsHookEx(hook_keyboard) {
                // Fatal error
                log::error!(
                    " UnhookWindowsHookEx keyboard, error {}",
                    Error::last_os_error()
                );
            }
            tx.send(format!(
                " SetWindowsHookExA mouse, error {}",
                Error::last_os_error()
            ))?;
            return Ok(invalid_ret);
        }

        *cur_hook_thread_id = GetCurrentThreadId();
        tx.send("".to_owned())?;
        return Ok((hook_keyboard, hook_mouse));
    }
}

pub fn hook() -> ResultType<()> {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let hook_keyboard;
        let hook_mouse;
        unsafe {
            match do_hook(tx.clone()) {
                Ok(hooks) => {
                    hook_keyboard = hooks.0;
                    hook_mouse = hooks.1;
                }
                Err(e) => {
                    // Fatal error
                    allow_err!(tx.send(format!("Unexpected err when hook {}", e)));
                    return;
                }
            }
            if hook_keyboard.is_null() {
                return;
            }

            let mut msg = MSG {
                hwnd: NULL as _,
                message: 0 as _,
                wParam: 0 as _,
                lParam: 0 as _,
                time: 0 as _,
                pt: POINT {
                    x: 0 as _,
                    y: 0 as _,
                },
            };
            while FALSE != GetMessageA(&mut msg, NULL as _, 0, 0) {
                if msg.message == WM_USER_EXIT_HOOK {
                    break;
                }

                TranslateMessage(&msg);
                DispatchMessageA(&msg);
            }

            if FALSE == UnhookWindowsHookEx(hook_keyboard as _) {
                // Fatal error
                log::error!(
                    "Failed UnhookWindowsHookEx keyboard, error {}",
                    Error::last_os_error()
                );
            }

            if FALSE == UnhookWindowsHookEx(hook_mouse as _) {
                // Fatal error
                log::error!(
                    "Failed UnhookWindowsHookEx mouse, error {}",
                    Error::last_os_error()
                );
            }

            *CUR_HOOK_THREAD_ID.lock().unwrap() = 0;
        }
    });

    match rx.recv() {
        Ok(msg) => {
            if msg == "" {
                Ok(())
            } else {
                bail!(msg)
            }
        }
        Err(e) => {
            bail!("Failed to wait hook result {}", e)
        }
    }
}

pub fn unhook() -> ResultType<()> {
    unsafe {
        let cur_hook_thread_id = CUR_HOOK_THREAD_ID.lock().unwrap();
        if *cur_hook_thread_id != 0 {
            if FALSE == PostThreadMessageA(*cur_hook_thread_id, WM_USER_EXIT_HOOK, 0, 0) {
                bail!(
                    "Failed to post message to exit hook, error {}",
                    Error::last_os_error()
                );
            }
        }
    }
    Ok(())
}

#[no_mangle]
pub extern "system" fn privacy_mode_hook_keyboard(
    code: c_int,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if code < 0 {
        unsafe {
            return CallNextHookEx(NULL as _, code, w_param, l_param);
        }
    }

    let ks = l_param as PKBDLLHOOKSTRUCT;
    let w_param2 = w_param as UINT;

    unsafe {
        let tu_enigo = (*ks).dwExtraInfo == enigo::ENIGO_INPUT_EXTRA_VALUE;
        let bay_gio = bay_gio_ms();
        if tu_enigo {
            PHIM_TU_XA_GAN_NHAT_MS.store(bay_gio, Ordering::Relaxed);
        } else {
            // iatoi-rc: bo go (Unikey...) tu SendInput Backspace/Unicode sau phim tu xa.
            // Cho qua phim injected trong IME_CUA_SO_MS sau phim tu xa (xem win_input_ime).
            let injected = (*ks).flags & LLKHF_INJECTED == LLKHF_INJECTED;
            let alt_down = (*ks).flags & LLKHF_ALTDOWN == LLKHF_ALTDOWN;
            let gan_nhat = PHIM_TU_XA_GAN_NHAT_MS.load(Ordering::Relaxed);
            let cho_phep_ime = injected
                && !alt_down
                && ime::tuy_chon_ime_bat(&hbb_common::config::Config::get_option(
                    ime::TUY_CHON_PRIVACY_ALLOW_IME,
                ));
            let phim = ime::PhimVao {
                tu_enigo,
                alt_down,
                injected,
                keydown: w_param2 == WM_KEYDOWN,
                vk_code: (*ks).vkCode,
                ms_tu_phim_tu_xa: if gan_nhat == 0 {
                    None
                } else {
                    Some(bay_gio.saturating_sub(gan_nhat))
                },
                cho_phep_ime,
            };
            // Disable alt key (Alt + Tab will switch windows) and all keys other than
            // P and Ctrl, except IME keys allowed above.
            if !ime::nen_cho_qua(&phim) {
                return 1;
            }

            match w_param2 {
                WM_KEYDOWN => {
                    if ime::PHIM_CHO_QUA_GOC.contains(&(*ks).vkCode) {
                        // NOTE: GetKeyboardState may not work well...

                        // Check if Ctrl + P is pressed
                        let cltr_down = (GetKeyState(VK_CONTROL) as u16) & (0x8000 as u16) > 0;
                        let key = LOBYTE((*ks).vkCode as _);
                        if cltr_down && (key == 'p' as u8 || key == 'P' as u8) {
                            // Ctrl + P is pressed, turn off privacy mode
                            if let Some(Err(e)) = super::turn_off_privacy(
                                super::INVALID_PRIVACY_MODE_CONN_ID,
                                Some(super::PrivacyModeState::OffByPeer),
                            ) {
                                log::error!("Failed to off_privacy {}", e);
                            }
                            // iatoi-rc: nguoi tai may thoat Privacy Mode bang Ctrl+P se chi thay
                            // desktop. Neu bat lock-on-privacy-exit='Y' thi khoa may luon (ve man
                            // hinh Lock Windows, muon vao phai co mat khau) -> dong lo hong Ctrl+P.
                            if hbb_common::config::Config::get_option("lock-on-privacy-exit")
                                == "Y"
                            {
                                crate::platform::lock_screen();
                            }
                        }
                    } else {
                        log::trace!("IME key allowed {}", (*ks).vkCode);
                    }
                }
                WM_KEYUP => {
                    log::trace!("WM_KEYUP {}", (*ks).vkCode);
                }
                _ => {
                    log::trace!("KEYBOARD OTHER {} {}", w_param2, (*ks).vkCode);
                }
            }
        }
    }
    unsafe { CallNextHookEx(NULL as _, code, w_param, l_param) }
}

#[no_mangle]
pub extern "system" fn privacy_mode_hook_mouse(
    code: c_int,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if code < 0 {
        unsafe {
            return CallNextHookEx(NULL as _, code, w_param, l_param);
        }
    }

    let ms = l_param as PMOUSEHOOKSTRUCT;
    unsafe {
        if (*ms).dwExtraInfo != enigo::ENIGO_INPUT_EXTRA_VALUE {
            return 1;
        }
    }
    unsafe { CallNextHookEx(NULL as _, code, w_param, l_param) }
}
