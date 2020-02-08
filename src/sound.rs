use swf_types as swf;

#[derive(Copy, Clone)]
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

impl<'a> From<&'a swf::tags::SoundStreamBlock> for Mp3StreamBlock<'a> {
    fn from(block: &'a swf::tags::SoundStreamBlock) -> Self {
        let mut data = &block.data[..];
        let samples = u16::from_le_bytes([data[0], data[1]]);
        data = &data[2..];
        Mp3StreamBlock {
            samples,
            mp3: Mp3::parse(data),
        }
    }
}

pub struct Sound<'a> {
    pub sample_rate: swf::SoundRate,
    pub stereo: bool,
    pub samples: u32,

    pub mp3: Option<Mp3<'a>>,
}

impl<'a> From<&'a swf::tags::DefineSound> for Sound<'a> {
    fn from(sound: &'a swf::tags::DefineSound) -> Self {
        let mp3 = match sound.format {
            swf::AudioCodingFormat::Mp3 => Some(Mp3::parse(&sound.data)),
            _ => {
                eprintln!("Sound::from: unsupported format: {:?}", sound.format);
                None
            }
        };

        Sound {
            sample_rate: sound.sound_rate,
            stereo: sound.sound_type == swf::SoundType::Stereo,
            samples: sound.sample_count,

            mp3,
        }
    }
}
