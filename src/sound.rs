use crate::dictionary::CharacterId;
use swf_tree as swf;

pub struct Sound<'a> {
    /// How many times smaller than 44.1kHz is the sample rate?
    pub sample_rate_divider: u8,
    pub stereo: bool,
    pub samples: u32,

    pub mp3_seek_samples: u16,
    pub mp3_data: &'a [u8],
}

pub struct DefineSound<'a> {
    pub id: CharacterId,
    pub sound: Sound<'a>,
}

// HACK(eddyb) move this into swf-{tree,parser}.
impl<'a> DefineSound<'a> {
    pub fn try_parse(tag: &'a swf::tags::Unknown) -> Option<Self> {
        if tag.code != 14 {
            return None;
        }

        let id = CharacterId(u16::from_le_bytes([tag.data[0], tag.data[1]]));
        let flags = tag.data[2];
        let samples = u32::from_le_bytes([tag.data[3], tag.data[4], tag.data[5], tag.data[6]]);
        let data = &tag.data[7..];

        let format = flags >> 4;
        if format != 2 {
            eprintln!(
                "unsupported format: {} ({})",
                format,
                [
                    "uncompressed, native-endian",
                    "adpcm",
                    "mp3",
                    "uncompressed, little-endian",
                    "nellymoser @ 16 kHz",
                    "nellymoser @ 8 kHz",
                    "nellymoser",
                    "speex",
                ]
                .get(format as usize)
                .cloned()
                .unwrap_or("")
            );
            return None;
        }
        let mut mp3_data = data;

        let sample_rate_divider = 1 << (3 - ((flags >> 2) & 3));
        let stereo = (flags & 1) != 0;

        let mp3_seek_samples = u16::from_le_bytes([mp3_data[0], mp3_data[1]]);
        mp3_data = &mp3_data[2..];

        Some(DefineSound {
            id,
            sound: Sound {
                sample_rate_divider,
                stereo,
                samples,

                mp3_seek_samples,
                mp3_data,
            },
        })
    }
}

pub struct StartSound {
    pub id: CharacterId,
}

// HACK(eddyb) move this into swf-{tree,parser}.
impl StartSound {
    pub fn try_parse(tag: &swf::tags::Unknown) -> Option<Self> {
        if tag.code != 15 {
            return None;
        }

        let id = CharacterId(u16::from_le_bytes([tag.data[0], tag.data[1]]));
        let flags = tag.data[2];
        if flags != 0 {
            return None;
        }

        Some(StartSound { id })
    }
}
