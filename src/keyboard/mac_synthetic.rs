// Iatoi rc (2026-11-20): keep synthetic keys out of a remote session on a macOS controller.
//
// Vietnamese input tools on macOS (OpenKey, EVKey, GoTiengViet) run their own event tap. While
// typing Telex they swallow the physical key and post backspaces plus the composed character on
// keycode 0 (kVK_ANSI_A) from a private event source. Forwarded to the peer as keys, that is a
// stream of Backspace and "A" that the peer's own input method (Unikey) turns into garbage.
//
// Two parts, both behind one local option, `mac-ignore-synthetic-keys`, on unless set to "N":
// - the rdev grab sits at the HID tap (rdev::set_grab_at_hid), ahead of every session tap, so the
//   physical key reaches us before such a tool can swallow it;
// - a key whose source state is not HIDSystemState is not sent to the peer (`nen_gui_phim`).
//
// Pure functions only, so the decision can be tested on its own:
//   rustc --edition 2021 --test src/keyboard/mac_synthetic.rs -o <out> && <out>

/// Local option name. Anything but "N" means on.
pub const OPTION_MAC_IGNORE_SYNTHETIC_KEYS: &str = "mac-ignore-synthetic-keys";

/// kCGEventSourceStateHIDSystemState: the state id every hardware key event carries.
pub const SOURCE_STATE_HID_SYSTEM: i64 = 1;

/// Value of the local option -> on/off. Empty (never set) is on.
pub fn option_bat(value: &str) -> bool {
    value != "N"
}

/// Should a key event seen by the grab be sent to the peer?
///
/// `bo_qua_tong_hop`: the option is on. `source_state_id`: kCGEventSourceStateID of the event.
/// Only the state id decides: hardware keys always carry HIDSystemState, input tools post from a
/// Private (-1) or CombinedSession (0) source. The source pid is not used, so a tool posting
/// with a HIDSystemState source still gets through - the HID tap is what keeps those away.
pub fn nen_gui_phim(bo_qua_tong_hop: bool, source_state_id: i64) -> bool {
    if !bo_qua_tong_hop {
        return true;
    }
    source_state_id == SOURCE_STATE_HID_SYSTEM
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tuy_chon_mac_dinh_bat() {
        assert!(option_bat(""));
        assert!(option_bat("Y"));
        assert!(!option_bat("N"));
    }

    #[test]
    fn phim_vat_ly_luon_gui() {
        assert!(nen_gui_phim(true, SOURCE_STATE_HID_SYSTEM));
        assert!(nen_gui_phim(false, SOURCE_STATE_HID_SYSTEM));
    }

    #[test]
    fn phim_tong_hop_bi_bo_khi_bat() {
        // OpenKey: CGEventSourceCreate(kCGEventSourceStatePrivate) -> -1
        assert!(!nen_gui_phim(true, -1));
        // CombinedSessionState
        assert!(!nen_gui_phim(true, 0));
    }

    #[test]
    fn tat_tuy_chon_thi_nhu_ban_goc() {
        assert!(nen_gui_phim(false, -1));
        assert!(nen_gui_phim(false, 0));
    }
}
