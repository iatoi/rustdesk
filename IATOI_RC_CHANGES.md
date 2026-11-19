# Iatoi rc: changes relative to RustDesk 1.5.3

Iatoi rc is a modified version of RustDesk 1.5.3 (tag `1.5.3`), distributed
under the same license, GNU AGPL-3.0 (see `LICENCE`). The original copyright
notices of the RustDesk authors and contributors are kept unchanged. This
file lists the modifications, as the license requires.

Iatoi rc is not RustDesk and is not affiliated with or endorsed by the
RustDesk authors. Build targets: Windows x64 and macOS arm64.

## Modifications (2026-11-19)

4. Opt-in silent/privacy host options and descriptive privacy-mode labels
   - `src/tray.rs`: `hide-tray = 'Y'` is also read from runtime config
     (`Config::get_option("hide-tray") == "Y"`), allowing headless system-app
     operation without a tray icon when explicitly enabled (off by default).
   - `src/server/connection.rs`: two opt-in host-side config options (off by
     default): `lock-on-disconnect = 'Y'` locks the screen when a remote
     control session ends regardless of client toggle; `auto-privacy-mode = 'Y'`
     activates the default privacy mode (Mode 1) once a remote desktop session
     is authorized.
   - `src/lang/en.rs`, `src/lang/vi.rs`: descriptive labels for Privacy Mode 1
     (Blackout screen / phủ đen) and Mode 2 (Virtual display / màn hình ảo).
   - `.github/workflows/flutter-build.yml`: re-enabled `build-for-macOS` for
     Apple Silicon (`aarch64-apple-darwin`), shipping `Iatoi-rc.app` inside
     `iatoi-rc-<version>-aarch64.dmg`.

5. Brand icon replacement for Iatoi rc
   - Replaced upstream RustDesk icons (`flutter/assets/icon.svg`,
     `res/logo.svg`, `res/icon.png`, `res/mac-icon.png`, `res/icon.ico`,
     `res/tray-icon.ico`, `res/mac-tray-light-x2.png`,
     `res/mac-tray-dark-x2.png`,
     `flutter/windows/runner/resources/app_icon.ico`,
     `flutter/macos/Runner/AppIcon.icns`) with the Iatoi monogram icon so
     Iatoi rc is visually distinct from upstream RustDesk.

## Modifications (2026-11-18)

1. `.github/workflows/flutter-build.yml`
   - Reduced to the jobs needed for the Windows x86_64 Flutter build and its
     release upload (`.exe` and `.msi`).
   - Removed the Windows Sciter (x86) job, the Windows arm64 matrix entries,
     and the iOS, macOS, Android, Linux (all variants), AppImage, Flatpak and
     web jobs.
   - Removed the jobs that only publish metadata or archives for the removed
     platforms (`publish-signed-update-metadata`, `publish_unsigned`).
   - The build steps of the Windows x86_64 Flutter job are unchanged.

2. Software update check disabled (no connection to RustDesk update services)
   - `src/common.rs`: new constant `IATOI_RC_NO_UPSTREAM_UPDATE`;
     `check_software_update()` and `do_check_software_update()` return early.
   - `src/updater.rs`: the auto-update scheduler, `check_update()` and the
     macOS `check_update_as_root()` return early.
   - `src/updater/artifact.rs`: fetching signed update metadata from
     upstream releases is refused.
   - Setting the constant to `false` restores the upstream behaviour.

3. Application renamed to `iatoi-rc` so it installs next to upstream RustDesk
   - `src/common.rs`: new constant `IATOI_RC_APP_NAME` (`iatoi-rc`);
     `load_custom_client()` replaces the stock application name with it
     before anything else runs. Upstream derives from that name the config
     and log directories, IPC pipe names, Windows service name, registry
     keys, URL scheme, install directory, shortcuts and firewall rule.
   - `.github/workflows/flutter-build.yml`: the built executable is renamed
     to `iatoi-rc.exe`; the packer and `res/msi/preprocess.py` are pointed at
     it (`--app-name iatoi-rc`); the two release files are named
     `iatoi-rc-<version>-<arch>.exe` and `.msi`.
   - `flutter/windows/runner/Runner.rc`: `FileDescription`, `ProductName`,
     `InternalName` and `OriginalFilename` name Iatoi rc. `CompanyName` and
     `LegalCopyright` are unchanged.
   - Not renamed, still shared with an upstream RustDesk on the same machine:
     the privacy-mode helper `RuntimeBroker_rustdesk.exe` and
     `WindowInjection.dll`, the virtual display driver, the printer driver,
     `librustdesk.dll`, the LAN discovery port.

No other source file, license file, or copyright notice was changed in this
revision. The submodule `libs/hbb_common` is untouched.

## lock-on-privacy-exit (2026-11-19)
`src/privacy_mode/win_input.rs`: khi người tại máy bấm **Ctrl+P** để thoát Privacy Mode (Mode 1 Blackout), nếu `Config::get_option("lock-on-privacy-exit") == "Y"` thì gọi `crate::platform::lock_screen()` (LockWorkStation) ngay sau `turn_off_privacy` → máy về màn hình Lock Windows, muốn vào phải có mật khẩu. Đóng lỗ hổng "Ctrl+P lộ desktop" của Privacy Mode gốc. Cờ opt-in, mặc định tắt. Script `day-iatoi-rc-qua-ssh.py` đặt `'Y'` cho chế độ `may-rieng`, `'N'` cho `nguoi-dung`.

## privacy-allow-ime (2026-11-20)
`src/privacy_mode/win_input.rs` + module thuần mới `src/privacy_mode/win_input_ime.rs` (khai báo trong `src/privacy_mode.rs`): sửa lỗi gõ tiếng Việt (Unikey/Telex) từ xa khi Privacy Mode 1 (Blackout) bật — trước đây "phiên này chúng" ra "Phia a cha" vì hook bàn phím chặn mọi phím không mang `ENIGO_INPUT_EXTRA_VALUE`, kể cả Backspace/ký tự Unicode mà bộ gõ tự `SendInput` sau phím từ xa.

- Hook ghi thời điểm phím từ xa (enigo) gần nhất. Phím không phải từ enigo được cho qua **chỉ khi** đồng thời: cờ `LLKHF_INJECTED` bật (phần mềm gửi, không phải bàn phím vật lý), cách phím từ xa gần nhất ≤ `IME_CUA_SO_MS` = 250 ms, và tuỳ chọn bật.
- Tuỳ chọn `privacy-allow-ime` (`Config::get_option`): **mặc định bật**; chỉ giá trị `"N"` mới tắt (về hành vi gốc).
- Không đổi: phím vật lý tại máy vẫn bị chặn; phím có Alt vẫn bị chặn; Ctrl+P thoát Privacy Mode và `lock-on-privacy-exit` giữ nguyên.
- Đánh đổi an ninh: trong cửa sổ 250 ms sau mỗi phím từ xa, **bất kỳ phần mềm nào trên máy đích** gửi phím giả lập (SendInput) đều lọt qua hook (không chỉ bộ gõ). Phần mềm đã chạy trên máy đích vốn có nhiều cách khác để thao tác máy, nhưng ai cần chặn tuyệt đối thì đặt `privacy-allow-ime = 'N'` (khi đó gõ tiếng Việt bằng bộ gõ trên máy đích sẽ hỏng như cũ).

## mac-ignore-synthetic-keys (2026-11-20)
macOS controller with a Vietnamese input tool on (OpenKey/EVKey/GoTiengViet, Telex) typing into a
Windows peer with Unikey: the tool's session event tap swallowed physical keys and posted
Backspace + composed characters on keycode 0 from a private event source; the rdev grab
(session tap) forwarded those to the peer as Backspace/"A". Local option
`mac-ignore-synthetic-keys` (LocalConfig, on unless `'N'`, read once when the grab starts):
- `libs/rdev/`: rdev vendored at the locked rev `0904f23ce496e255461a98848b85c11fa5a62eab` (MIT,
  licence file kept), wired by `[patch."https://github.com/rustdesk-org/rdev"]` in `Cargo.toml`,
  excluded from the workspace; `Cargo.lock` drops the git `source` of `rdev`. Patched only in
  `src/macos/grab.rs` (+ re-exports in `src/macos/mod.rs`, `src/lib.rs`): `set_grab_at_hid(bool)`
  puts the grab tap at `kCGHIDEventTap` (falls back to the session tap if refused, logged);
  `last_event_source()` returns the current event's `kCGEventSourceStateID` / `UnixProcessID`.
- `src/keyboard/mac_synthetic.rs` (new): `nen_gui_phim(on, source_state_id)` — while a session
  holds the keyboard, a key whose source state is not HIDSystemState (1) is not sent to the peer.
- `src/keyboard.rs`: reads the option, calls `rdev::set_grab_at_hid`, applies `nen_gui_phim`.
Not covered: Input source 2 (Flutter key events) still sees what the input tool produced.
