use crate::dictionary::CharacterId;
use image::{DynamicImage, Rgb, RgbImage};
use swf_tree as swf;

pub struct DefineBitmap {
    pub id: CharacterId,
    pub image: DynamicImage,
}

// HACK(eddyb) move this into swf-{tree,parser}.
impl DefineBitmap {
    pub fn try_parse(tag: &swf::tags::Unknown) -> Option<Self> {
        if tag.code != 20 {
            return None;
        }

        let id = CharacterId(u16::from_le_bytes([tag.data[0], tag.data[1]]));
        let format = tag.data[2];
        let width = u16::from_le_bytes([tag.data[3], tag.data[4]]);
        let height = u16::from_le_bytes([tag.data[5], tag.data[6]]);

        let (color_table_len, compressed_data) = if format == 3 {
            (tag.data[7] as usize + 1, &tag.data[8..])
        } else {
            (0, &tag.data[7..])
        };

        let data = inflate::inflate_bytes_zlib(compressed_data).unwrap();
        let color_table = &data[..color_table_len * 3];
        let data = &data[color_table_len * 3..];

        // FIXME(eddyb) this is probably really inefficient.
        let px_to_rgb = |px: &[u8]| match format {
            3 => {
                let i = px[0] as usize * 3;
                [color_table[i], color_table[i + 1], color_table[i + 2]]
            }
            4 => {
                let rgb = u16::from_be_bytes([px[0], px[1]]);
                let (r, g, b) = (rgb >> 10, (rgb >> 5) & 0x1f, rgb & 0x1f);

                // Uniformly map a 5-bit channel to a 8-bit one by repeating
                // the top 3 bits below the original 5 bits, to turn e.g.
                // 0x00 into 0x00, 0x10 into 0x84 and 0x1f into 0xff.
                let extend = |x| ((x << 3) | (x >> 2)) as u8;

                [extend(r), extend(g), extend(b)]
            }
            5 => [px[0], px[1], px[2]],
            _ => unreachable!(),
        };

        let px_bytes = match format {
            3 => 1,
            4 => 2,
            5 => 4,
            _ => {
                eprintln!("unsupported bitmap format {}", format);
                return None;
            }
        };
        let row_len = (width as usize * px_bytes + 3) / 4 * 4;
        let image = RgbImage::from_fn(width as u32, height as u32, |x, y| {
            let i = y as usize * row_len + x as usize * px_bytes;
            Rgb(px_to_rgb(&data[i..i + px_bytes]))
        });

        Some(DefineBitmap {
            id,
            image: DynamicImage::ImageRgb8(image),
        })
    }
}
