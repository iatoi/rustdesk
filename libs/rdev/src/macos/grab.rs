#![allow(improper_ctypes_definitions)]
use crate::macos::common::*;
use crate::rdev::{Event, GrabError};
use cocoa::base::nil;
use cocoa::foundation::NSAutoreleasePool;
use core_graphics::event::{CGEventTapLocation, CGEventType};
use std::os::raw::c_void;

static mut GLOBAL_CALLBACK: Option<Box<dyn FnMut(Event) -> Option<Event>>> = None;

// Iatoi rc patch (2026-11-20).
// Vietnamese input tools on macOS (OpenKey, EVKey, GoTiengViet) install their own event tap at the
// session tap location, head-insert. Whichever head-insert session tap is created last runs first,
// and OpenKey re-creates its tap after every wake. When it runs ahead of this grab it swallows the
// physical key and posts backspaces plus characters on keycode 0 through CGEventTapPostEvent, so
// this grab saw synthetic keys instead of the ones typed. A tap at the HID location runs before
// every session tap and never sees what those tools post.
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};

// CGEventField values (CGEventTypes.h).
#[allow(non_upper_case_globals)]
const kCGEventSourceUnixProcessID: u32 = 41;
#[allow(non_upper_case_globals)]
const kCGEventSourceStateID: u32 = 45;

static GRAB_AT_HID: AtomicBool = AtomicBool::new(false);
static GRAB_IS_AT_HID: AtomicBool = AtomicBool::new(false);
static LAST_SOURCE_STATE_ID: AtomicI64 = AtomicI64::new(1);
static LAST_SOURCE_PID: AtomicI64 = AtomicI64::new(0);

/// Ask the next `grab` to put its tap at the HID location (falls back to the session location if
/// the system refuses). Must be called before `grab`. Off by default, matching upstream.
pub fn set_grab_at_hid(on: bool) {
    GRAB_AT_HID.store(on, Ordering::SeqCst);
}

/// Whether the running grab's tap actually sits at the HID location.
pub fn grab_is_at_hid() -> bool {
    GRAB_IS_AT_HID.load(Ordering::SeqCst)
}

/// Source of the event currently being handed to the grab callback:
/// (kCGEventSourceStateID, kCGEventSourceUnixProcessID). Only meaningful when read from inside
/// that callback, which runs synchronously on the grab thread. Hardware keys carry state id 1
/// (HIDSystemState); events posted by another process usually carry -1 (Private) or 0
/// (CombinedSessionState) and that process's pid.
pub fn last_event_source() -> (i64, i64) {
    (
        LAST_SOURCE_STATE_ID.load(Ordering::SeqCst),
        LAST_SOURCE_PID.load(Ordering::SeqCst),
    )
}

unsafe extern "C" fn raw_callback(
    _proxy: CGEventTapProxy,
    _type: CGEventType,
    cg_event: CGEventRef,
    _user_info: *mut c_void,
) -> CGEventRef {
    // println!("Event ref {:?}", cg_event_ptr);
    // let cg_event: CGEvent = transmute_copy::<*mut c_void, CGEvent>(&cg_event_ptr);
    if let Ok(mut state) = KEYBOARD_STATE.lock() {
        if let Some(keyboard) = state.as_mut() {
            if let Some(event) = convert(_type, &cg_event, keyboard) {
                LAST_SOURCE_STATE_ID.store(
                    cg_event.get_integer_value_field(kCGEventSourceStateID),
                    Ordering::SeqCst,
                );
                LAST_SOURCE_PID.store(
                    cg_event.get_integer_value_field(kCGEventSourceUnixProcessID),
                    Ordering::SeqCst,
                );
                if let Some(callback) = &mut GLOBAL_CALLBACK {
                    if callback(event).is_none() {
                        cg_event.set_type(CGEventType::Null);
                    }
                }
            }
        }
    }
    cg_event
}

static mut CUR_LOOP: CFRunLoopSourceRef = std::ptr::null_mut();

#[inline]
pub fn is_grabbed() -> bool {
    unsafe {
        !CUR_LOOP.is_null()
    }
}

pub fn grab<T>(callback: T) -> Result<(), GrabError>
where
    T: FnMut(Event) -> Option<Event> + 'static,
{
    if is_grabbed() {
        return Ok(());
    }

    unsafe {
        GLOBAL_CALLBACK = Some(Box::new(callback));
        let _pool = NSAutoreleasePool::new(nil);
        let mut at_hid = GRAB_AT_HID.load(Ordering::SeqCst);
        let mut tap: CFMachPortRef = std::ptr::null();
        if at_hid {
            tap = CGEventTapCreate(
                CGEventTapLocation::HID,
                kCGHeadInsertEventTap,
                CGEventTapOption::Default,
                kCGEventMaskForAllEvents,
                raw_callback,
                nil,
            );
            if tap.is_null() {
                log::error!("rdev grab: HID event tap refused, falling back to the session tap");
                at_hid = false;
            }
        }
        if tap.is_null() {
            tap = CGEventTapCreate(
                CGEventTapLocation::Session, // HID, Session, AnnotatedSession,
                kCGHeadInsertEventTap,
                CGEventTapOption::Default,
                kCGEventMaskForAllEvents,
                raw_callback,
                nil,
            );
        }
        if tap.is_null() {
            return Err(GrabError::EventTapError);
        }
        GRAB_IS_AT_HID.store(at_hid, Ordering::SeqCst);
        log::info!("rdev grab: event tap at {}", if at_hid { "HID" } else { "session" });
        let _loop = CFMachPortCreateRunLoopSource(nil, tap, 0);
        if _loop.is_null() {
            return Err(GrabError::LoopSourceError);
        }

        CUR_LOOP = CFRunLoopGetCurrent() as _;
        CFRunLoopAddSource(CUR_LOOP, _loop, kCFRunLoopCommonModes);

        CGEventTapEnable(tap, true);
        CFRunLoopRun();
    }
    Ok(())
}

pub fn exit_grab() -> Result<(), GrabError> {
    unsafe {
        if !CUR_LOOP.is_null() {
            CFRunLoopStop(CUR_LOOP);
            CUR_LOOP = std::ptr::null_mut();
        }
    }
    Ok(())
}
