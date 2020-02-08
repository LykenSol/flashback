use image::{DynamicImage, Rgb, RgbImage, Rgba, RgbaImage};
use swf_types as swf;

pub struct Bitmap {
    pub image: DynamicImage,
}

impl<'a> From<&'a swf::tags::DefineBitmap> for Bitmap {
    fn from(bitmap: &swf::tags::DefineBitmap) -> Self {
        let has_alpha = match bitmap.media_type {
            swf::ImageType::SwfLossless1 => false,
            swf::ImageType::SwfLossless2 => true,
            _ => {
                eprintln!("Bitmap::from: unsupported type: {:?}", bitmap.media_type);

                return Bitmap {
                    image: DynamicImage::ImageRgb8(RgbImage::new(
                        bitmap.width as u32,
                        bitmap.height as u32,
                    )),
                };
            }
        };

        let format = bitmap.data[0];
        let width = u16::from_le_bytes([bitmap.data[1], bitmap.data[2]]);
        let height = u16::from_le_bytes([bitmap.data[3], bitmap.data[4]]);

        let (color_table_len, compressed_data) = if format == 3 {
            (bitmap.data[5] as usize + 1, &bitmap.data[6..])
        } else {
            (0, &bitmap.data[5..])
        };

        let data = inflate::inflate_bytes_zlib(compressed_data).unwrap();

        let (color_table, data) = data.split_at(color_table_len * (3 + has_alpha as usize));

        // FIXME(eddyb) this is probably really inefficient.
        let rgb_px = |px: &[u8]| {
            let px = match format {
                3 => {
                    let i = px[0] as usize * 3;

                    &color_table[i..i + 3]
                }
                4 => {
                    let rgb = u16::from_be_bytes([px[0], px[1]]);
                    let (r, g, b) = (rgb >> 10, (rgb >> 5) & 0x1f, rgb & 0x1f);

                    // Uniformly map a 5-bit channel to a 8-bit one by repeating
                    // the top 3 bits below the original 5 bits, to turn e.g.
                    // 0x00 into 0x00, 0x10 into 0x84 and 0x1f into 0xff.
                    let extend = |x| ((x << 3) | (x >> 2)) as u8;

                    return Rgb([extend(r), extend(g), extend(b)]);
                }
                5 => px,
                _ => unreachable!(),
            };
            Rgb([px[0], px[1], px[2]])
        };
        let rgba_px = |px: &[u8]| {
            let px = match format {
                3 => {
                    let i = px[0] as usize * 4;
                    &color_table[i..i + 4]
                }
                5 => px,
                _ => unreachable!(),
            };
            Rgba([px[0], px[1], px[2], px[3]])
        };

        let px_bytes = match format {
            3 => 1,
            4 => 2,
            5 => 4,
            _ => {
                eprintln!("Bitmap::from: unsupported bitmap format {}", format);

                return Bitmap {
                    image: DynamicImage::ImageRgb8(RgbImage::new(
                        bitmap.width as u32,
                        bitmap.height as u32,
                    )),
                };
            }
        };
        let row_len = (width as usize * px_bytes + 3) / 4 * 4;
        let image = if has_alpha {
            // FIXME(eddyb) figure out how to deduplicate all of this.
            DynamicImage::ImageRgba8(RgbaImage::from_fn(width as u32, height as u32, |x, y| {
                let i = y as usize * row_len + x as usize * px_bytes;
                rgba_px(&data[i..i + px_bytes])
            }))
        } else {
            DynamicImage::ImageRgb8(RgbImage::from_fn(width as u32, height as u32, |x, y| {
                let i = y as usize * row_len + x as usize * px_bytes;
                rgb_px(&data[i..i + px_bytes])
            }))
        };

        Bitmap { image }
    }
}
