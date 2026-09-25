use std::str::Bytes;

pub struct RawGif {
    pub width: u16,
    pub height: u16,
    pub global_color_table: Vec<(u8, u8, u8)>,
    pub frames: Vec<GifFrame>,
}

pub struct GifFrame {
    pub left: u16,
    pub top: u16,
    pub width: u16,
    pub height: u16,
    pub has_local_color_table: bool,
    pub local_color_table: Vec<(u8, u8, u8)>,
    pub compressed_pixels: Vec<u8>,
}

impl RawGif {
    pub fn parse(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.len() < 13 {
            return Err("File too short");
        }

        // 1. Validate Header (GIF87a or GIF89a)
        if &bytes[0..3] != b"GIF" {
            return Err("Not a GIF file");
        }
        let mut cursor = 6;

        // 2. Logical Screen Descriptor
        let width = u16::from_le_bytes([bytes[cursor], bytes[cursor + 1]]);
        let height = u16::from_le_bytes([bytes[cursor + 2], bytes[cursor + 3]]);
        let packed = bytes[cursor + 4];
        cursor += 7;

        // 3. Global Color Table
        let has_gct = (packed & 0x80) != 0;
        let gct_size = 2usize.pow(((packed & 0x07) + 1) as u32);
        let mut global_color_table = Vec::new();

        if has_gct {
            for _ in 0..gct_size {
                if cursor + 3 > bytes.len() {
                    return Err("Unexpected EOF in GCT");
                }
                global_color_table.push((bytes[cursor], bytes[cursor + 1], bytes[cursor + 2]));
                cursor += 3;
            }
        }

        let mut frames = Vec::new();

        // 4. Data Block Loop
        while cursor < bytes.len() {
            match bytes[cursor] {
                0x21 => {
                    // Extension Introducer (Graphics Control, Application, etc.)
                    cursor += 1;
                    if cursor >= bytes.len() {
                        break;
                    }
                    let ext_label = bytes[cursor];
                    cursor += 1;

                    // Skip extension sub-blocks
                    while cursor < bytes.len() {
                        let block_size = bytes[cursor] as usize;
                        if block_size == 0 {
                            cursor += 1;
                            break;
                        }
                        cursor += 1 + block_size;
                    }
                }
                0x2C => {
                    // Image Descriptor (Start of a Frame)
                    if cursor + 10 > bytes.len() {
                        return Err("Unexpected EOF in Image Descriptor");
                    }
                    let left = u16::from_le_bytes([bytes[cursor + 1], bytes[cursor + 2]]);
                    let top = u16::from_le_bytes([bytes[cursor + 3], bytes[cursor + 4]]);
                    let f_width = u16::from_le_bytes([bytes[cursor + 5], bytes[cursor + 6]]);
                    let f_height = u16::from_le_bytes([bytes[cursor + 7], bytes[cursor + 8]]);
                    let f_packed = bytes[cursor + 9];
                    cursor += 10;

                    // Frame Local Color Table
                    let has_lct = (f_packed & 0x80) != 0;
                    let mut local_color_table = Vec::new();
                    if has_lct {
                        let lct_size = 2usize.pow(((f_packed & 0x07) + 1) as u32);
                        for _ in 0..lct_size {
                            local_color_table.push((
                                bytes[cursor],
                                bytes[cursor + 1],
                                bytes[cursor + 2],
                            ));
                            cursor += 3;
                        }
                    }

                    // Read LZW Minimum Code Size
                    let _lzw_min_code_size = bytes[cursor];
                    cursor += 1;

                    // Gather compressed data sub-blocks
                    let mut compressed_pixels = Vec::new();
                    while cursor < bytes.len() {
                        let block_size = bytes[cursor] as usize;
                        cursor += 1;
                        if block_size == 0 {
                            break;
                        } // Block Terminator

                        compressed_pixels.extend_from_slice(&bytes[cursor..cursor + block_size]);
                        cursor += block_size;
                    }

                    frames.push(GifFrame {
                        left,
                        top,
                        width: f_width,
                        height: f_height,
                        has_local_color_table: has_lct,
                        local_color_table,
                        compressed_pixels, // Note: This must be uncompressed via LZW to get final RGB pixels
                    });
                }
                0x3B => break,    // Trailer (End of GIF file)
                _ => cursor += 1, // Skip unknown/corrupt padding
            }
        }

        Ok(RawGif {
            width,
            height,
            global_color_table,
            frames,
        })
    }
}
