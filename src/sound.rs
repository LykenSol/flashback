use crate::dictionary::CharacterId;
use swf_tree as swf;

pub struct Mp3<'a> {
    pub seek_samples: u16,
    pub data: &'a [u8],
}

impl<'a> Mp3<'a> {
    fn parse(mut data: &'a [u8]) -> Self {
        // FIXME(eddyb) process all the mp3 frames correctly.
        let seek_samples = u16::from_le_bytes([data[0], data[1]]);
        data = &data[2..];
        Mp3 { seek_samples, data }
    }
}

pub struct Mp3StreamBlock<'a> {
    pub samples: u16,
    pub mp3: Mp3<'a>,
}

impl<'a> Mp3StreamBlock<'a> {
    fn parse(mut data: &'a [u8]) -> Self {
        let samples = u16::from_le_bytes([data[0], data[1]]);
        data = &data[2..];
        Mp3StreamBlock {
            samples,
            mp3: Mp3::parse(data),
        }
    }
}

pub struct Sound<'a> {
    /// How many times smaller than 44.1kHz is the sample rate?
    pub sample_rate_divider: u8,
    pub stereo: bool,
    pub samples: u32,

    pub mp3: Mp3<'a>,
}

pub struct DefineSound<'a> {
    pub id: CharacterId,
    pub sound: Sound<'a>,
}

const FORMATS: &[&str] = &[
    "uncompressed, native-endian",
    "adpcm",
    "mp3",
    "uncompressed, little-endian",
    "nellymoser @ 16 kHz",
    "nellymoser @ 8 kHz",
    "nellymoser",
    "speex",
];

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
                "DefineSound::try_parse: unsupported format: {} ({})",
                format,
                FORMATS.get(format as usize).cloned().unwrap_or("")
            );
            return None;
        }

        let sample_rate_divider = 1 << (3 - ((flags >> 2) & 3));
        let stereo = (flags & 1) != 0;

        Some(DefineSound {
            id,
            sound: Sound {
                sample_rate_divider,
                stereo,
                samples,

                mp3: Mp3::parse(data),
            },
        })
    }
}

#[derive(Debug)]
pub struct StartSound {
    pub character: CharacterId,
    pub no_restart: bool,
    pub loops: bool,
}

// HACK(eddyb) move this into swf-{tree,parser}.
impl StartSound {
    pub fn try_parse(tag: &swf::tags::Unknown) -> Option<Self> {
        if tag.code != 15 {
            return None;
        }

        let character = CharacterId(u16::from_le_bytes([tag.data[0], tag.data[1]]));
        let flags = tag.data[2];
        if (flags & !0x10) != 0 {
            eprintln!(
                "StartSound::try_parse: unsupported SoundInfo: {:?}",
                &tag.data[2..]
            );
        }

        let no_restart = (flags & 0x10) != 0;
        let loops = (flags & 4) != 0;

        Some(StartSound {
            character,
            no_restart,
            loops,
        })
    }
}

pub struct SoundStreamHead {
    pub average_samples: u16,
}

// HACK(eddyb) move this into swf-{tree,parser}.
impl SoundStreamHead {
    pub fn try_parse(tag: &swf::tags::Unknown) -> Option<Self> {
        if tag.code != 18 {
            return None;
        }

        let flags = tag.data[1];
        let average_samples = u16::from_le_bytes([tag.data[2], tag.data[3]]);

        let format = flags >> 4;
        if format != 2 {
            eprintln!(
                "SoundStreamHead::try_parse: unsupported format: {} ({})",
                format,
                FORMATS.get(format as usize).cloned().unwrap_or("")
            );
            return None;
        }

        Some(SoundStreamHead { average_samples })
    }
}

#[derive(Debug)]
pub struct SoundStreamBlock<'a> {
    data: &'a [u8],
}

// HACK(eddyb) move this into swf-{tree,parser}.
impl<'a> SoundStreamBlock<'a> {
    pub fn try_parse(tag: &'a swf::tags::Unknown) -> Option<Self> {
        if tag.code != 19 {
            return None;
        }
        Some(SoundStreamBlock { data: &tag.data })
    }

    pub fn as_mp3(&self) -> Mp3StreamBlock<'a> {
        Mp3StreamBlock::parse(self.data)
    }
}
