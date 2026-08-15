/// Maps Linux / XKB key scancodes to Windows RDP virtual scan codes
pub fn xkb_to_rdp_scancode(xkb_code: u32) -> (u16, bool) {
    // Standard PC keyboard offset: XKB scancodes are usually Evdev code + 8
    let evdev = if xkb_code >= 8 { xkb_code - 8 } else { xkb_code };

    match evdev {
        // Alphanumeric keys
        1 => (0x01, false),  // Escape
        2 => (0x02, false),  // 1
        3 => (0x03, false),  // 2
        4 => (0x04, false),  // 3
        5 => (0x05, false),  // 4
        6 => (0x06, false),  // 5
        7 => (0x07, false),  // 6
        8 => (0x08, false),  // 7
        9 => (0x09, false),  // 8
        10 => (0x0A, false), // 9
        11 => (0x0B, false), // 0
        12 => (0x0C, false), // Minus
        13 => (0x0D, false), // Equal
        14 => (0x0E, false), // Backspace
        15 => (0x0F, false), // Tab
        16 => (0x10, false), // Q
        17 => (0x11, false), // W
        18 => (0x12, false), // E
        19 => (0x13, false), // R
        20 => (0x14, false), // T
        21 => (0x15, false), // Y
        22 => (0x16, false), // U
        23 => (0x17, false), // I
        24 => (0x18, false), // O
        25 => (0x19, false), // P
        28 => (0x1C, false), // Enter
        29 => (0x1D, false), // Left Ctrl
        30 => (0x1E, false), // A
        31 => (0x1F, false), // S
        32 => (0x20, false), // D
        33 => (0x21, false), // F
        34 => (0x22, false), // G
        35 => (0x23, false), // H
        36 => (0x24, false), // J
        37 => (0x25, false), // K
        38 => (0x26, false), // L
        42 => (0x2A, false), // Left Shift
        44 => (0x2C, false), // Z
        45 => (0x2D, false), // X
        46 => (0x2E, false), // C
        47 => (0x2F, false), // V
        48 => (0x30, false), // B
        49 => (0x31, false), // N
        50 => (0x32, false), // M
        54 => (0x36, false), // Right Shift
        56 => (0x38, false), // Left Alt
        57 => (0x39, false), // Space
        58 => (0x3A, false), // CapsLock

        // Function keys
        59 => (0x3B, false), // F1
        60 => (0x3C, false), // F2
        61 => (0x3D, false), // F3
        62 => (0x3E, false), // F4
        63 => (0x3F, false), // F5
        64 => (0x40, false), // F6
        65 => (0x41, false), // F7
        66 => (0x42, false), // F8
        67 => (0x43, false), // F9
        68 => (0x44, false), // F10
        87 => (0x57, false), // F11
        88 => (0x58, false), // F12

        // Extended keys (prefixed with E0 in RDP)
        96 => (0x1C, true),  // Keypad Enter
        97 => (0x1D, true),  // Right Ctrl
        100 => (0x38, true), // Right Alt / AltGr
        102 => (0x47, true), // Home
        103 => (0x48, true), // Up Arrow
        104 => (0x49, true), // Page Up
        105 => (0x4B, true), // Left Arrow
        106 => (0x4D, true), // Right Arrow
        107 => (0x4F, true), // End
        108 => (0x50, true), // Down Arrow
        109 => (0x51, true), // Page Down
        110 => (0x52, true), // Insert
        111 => (0x53, true), // Delete
        125 => (0x5B, true), // Left Super / Windows Key
        126 => (0x5C, true), // Right Super
        127 => (0x5D, true), // Menu / Apps Key

        _ => (evdev as u16, false),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scancode_translation() {
        // Evdev Enter (28) -> RDP (0x1C, false)
        assert_eq!(xkb_to_rdp_scancode(36), (0x1C, false)); // 28 + 8 = 36 in XKB
        // Super / Win key (125) -> (0x5B, true)
        assert_eq!(xkb_to_rdp_scancode(133), (0x5B, true)); // 125 + 8 = 133 in XKB
    }
}
