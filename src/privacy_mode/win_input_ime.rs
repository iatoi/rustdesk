//! iatoi-rc: logic quyet dinh cho/chan phim trong hook ban phim cua Privacy Mode 1
//! (Blackout). Ham thuan, khong goi WinAPI, de test duoc tren moi nen tang.
//!
//! Van de: bo go (Unikey/Telex...) tren may dich nuot phim tu xa roi tu SendInput
//! Backspace + ky tu Unicode voi dwExtraInfo cua no. Hook goc chan moi phim khong
//! mang ENIGO_INPUT_EXTRA_VALUE nen chu bi mat dau, mat chu.
//! Cach sua: cho qua phim do phan mem gui (LLKHF_INJECTED) neu den trong
//! `IME_CUA_SO_MS` sau phim tu xa gan nhat va tuy chon `privacy-allow-ime` khong
//! phai "N". Phim vat ly tai may van bi chan; Alt van bi chan.

/// Cua so thoi gian (ms) sau phim tu xa gan nhat, trong do phim injected
/// (vd tu bo go tieng Viet) duoc cho qua.
pub const IME_CUA_SO_MS: u64 = 250;

/// Ten tuy chon cau hinh. Mac dinh bat; chi gia tri "N" moi tat.
pub const TUY_CHON_PRIVACY_ALLOW_IME: &str = "privacy-allow-ime";

/// vkCode cua P, LCtrl, RCtrl: phim duoc cho qua tu goc de nguoi tai may bam Ctrl+P.
pub const PHIM_CHO_QUA_GOC: [u32; 3] = [80, 162, 163];

/// Thong tin mot su kien phim, da tach khoi KBDLLHOOKSTRUCT.
#[derive(Debug, Clone, Copy)]
pub struct PhimVao {
    /// dwExtraInfo == ENIGO_INPUT_EXTRA_VALUE (phim tu xa do Iatoi rc gui).
    pub tu_enigo: bool,
    /// Co LLKHF_ALTDOWN.
    pub alt_down: bool,
    /// Co LLKHF_INJECTED (phim do phan mem gui, khong phai ban phim vat ly).
    pub injected: bool,
    /// w_param == WM_KEYDOWN.
    pub keydown: bool,
    pub vk_code: u32,
    /// So ms tu phim tu xa gan nhat; None neu chua co phim tu xa nao.
    pub ms_tu_phim_tu_xa: Option<u64>,
    /// Tuy chon privacy-allow-ime dang bat.
    pub cho_phep_ime: bool,
}

/// Tuy chon bat tru khi gia tri dung bang "N".
pub fn tuy_chon_ime_bat(gia_tri: &str) -> bool {
    gia_tri != "N"
}

/// Phim khong phai tu enigo nhung la phim bo go gui ngay sau phim tu xa.
pub fn la_phim_ime(p: &PhimVao) -> bool {
    !p.tu_enigo
        && p.cho_phep_ime
        && p.injected
        && matches!(p.ms_tu_phim_tu_xa, Some(d) if d <= IME_CUA_SO_MS)
}

/// true = goi CallNextHookEx (cho qua); false = tra 1 (chan).
pub fn nen_cho_qua(p: &PhimVao) -> bool {
    if p.tu_enigo {
        return true;
    }
    // Alt + Tab chuyen cua so: van chan nhu goc.
    if p.alt_down {
        return false;
    }
    if la_phim_ime(p) {
        return true;
    }
    // Hanh vi goc: KEYDOWN chi cho P va Ctrl; KEYUP va loai khac cho qua.
    if p.keydown && !PHIM_CHO_QUA_GOC.contains(&p.vk_code) {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    const VK_A: u32 = 0x41;
    const VK_BACK: u32 = 0x08;
    const VK_PACKET: u32 = 0xE7;

    fn phim(vk: u32) -> PhimVao {
        PhimVao {
            tu_enigo: false,
            alt_down: false,
            injected: false,
            keydown: true,
            vk_code: vk,
            ms_tu_phim_tu_xa: Some(10),
            cho_phep_ime: true,
        }
    }

    #[test]
    fn phim_tu_xa_luon_qua() {
        let p = PhimVao { tu_enigo: true, alt_down: true, ..phim(VK_A) };
        assert!(nen_cho_qua(&p));
    }

    #[test]
    fn phim_vat_ly_bi_chan_du_vua_co_phim_tu_xa() {
        assert!(!nen_cho_qua(&phim(VK_A)));
        assert!(!nen_cho_qua(&PhimVao { ms_tu_phim_tu_xa: Some(0), ..phim(VK_BACK) }));
    }

    #[test]
    fn injected_trong_250ms_qua() {
        for vk in [VK_A, VK_BACK, VK_PACKET] {
            let p = PhimVao { injected: true, ..phim(vk) };
            assert!(nen_cho_qua(&p), "vk {vk}");
        }
        let bien = PhimVao { injected: true, ms_tu_phim_tu_xa: Some(IME_CUA_SO_MS), ..phim(VK_A) };
        assert!(nen_cho_qua(&bien));
    }

    #[test]
    fn injected_sau_250ms_bi_chan() {
        let p = PhimVao { injected: true, ms_tu_phim_tu_xa: Some(IME_CUA_SO_MS + 1), ..phim(VK_A) };
        assert!(!nen_cho_qua(&p));
    }

    #[test]
    fn injected_khi_chua_co_phim_tu_xa_bi_chan() {
        let p = PhimVao { injected: true, ms_tu_phim_tu_xa: None, ..phim(VK_A) };
        assert!(!nen_cho_qua(&p));
    }

    #[test]
    fn tat_tuy_chon_thi_chan() {
        assert!(!tuy_chon_ime_bat("N"));
        assert!(tuy_chon_ime_bat(""));
        assert!(tuy_chon_ime_bat("Y"));
        let p = PhimVao { injected: true, cho_phep_ime: tuy_chon_ime_bat("N"), ..phim(VK_A) };
        assert!(!nen_cho_qua(&p));
    }

    #[test]
    fn alt_van_bi_chan_ke_ca_injected() {
        let p = PhimVao { injected: true, alt_down: true, ..phim(VK_A) };
        assert!(!nen_cho_qua(&p));
        assert!(!nen_cho_qua(&PhimVao { alt_down: true, ..phim(80) }));
    }

    #[test]
    fn p_va_ctrl_nhu_cu() {
        for vk in PHIM_CHO_QUA_GOC {
            let p = PhimVao { ms_tu_phim_tu_xa: None, cho_phep_ime: false, ..phim(vk) };
            assert!(nen_cho_qua(&p), "vk {vk}");
        }
    }

    #[test]
    fn keyup_vat_ly_qua_nhu_cu() {
        let p = PhimVao { keydown: false, ms_tu_phim_tu_xa: None, ..phim(VK_A) };
        assert!(nen_cho_qua(&p));
    }
}
