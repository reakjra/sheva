use std::io::Cursor;

use anyhow::{Context, Result, anyhow};
use pipewire::core::CoreRc;
use pipewire::properties::PropertiesBox;
use pipewire::spa::param::ParamType;
use pipewire::spa::param::audio::{AudioFormat, AudioInfoRaw};
use pipewire::spa::pod::serialize::PodSerializer;
use pipewire::spa::pod::{Object, Pod, Value};
use pipewire::spa::sys::SPA_AUDIO_MAX_CHANNELS;
use pipewire::spa::utils::{Direction, SpaTypes};
use pipewire::stream::{StreamFlags, StreamListener, StreamRc};

const MAX_CHANNELS: usize = SPA_AUDIO_MAX_CHANNELS as usize;

pub struct Format {
    pub rate: u32,
    pub positions: &'static [u32],
}

pub struct Sink<F> {
    _stream: StreamRc,
    _listener: StreamListener<F>,
}

impl<F: FnMut(&[f32]) + 'static> Sink<F> {
    pub fn register(
        core: CoreRc,
        name: &str,
        properties: PropertiesBox,
        format: &Format,
        on_frame: F,
    ) -> Result<Self> {
        let channels = format.positions.len();
        let stream = StreamRc::new(core, name, properties)?;
        let listener = stream
            .add_local_listener_with_user_data(on_frame)
            .process(move |stream, on_frame| {
                let Some(mut buffer) = stream.dequeue_buffer() else {
                    return;
                };
                let Some(data) = buffer.datas_mut().first_mut() else {
                    return;
                };
                let size = data.chunk().size() as usize;
                let Some(bytes) = data.data() else {
                    return;
                };
                let (samples, _) = bytes[..size].as_chunks::<4>();
                let mut frame = [0.0; MAX_CHANNELS];
                for frame_bytes in samples.chunks_exact(channels) {
                    for (sample, raw) in frame.iter_mut().zip(frame_bytes) {
                        *sample = f32::from_le_bytes(*raw);
                    }
                    on_frame(&frame[..channels]);
                }
            })
            .register()?;

        let pod = format_pod(format)?;
        stream.connect(
            Direction::Input,
            None,
            StreamFlags::MAP_BUFFERS | StreamFlags::RT_PROCESS,
            &mut [Pod::from_bytes(&pod).context("format pod is malformed")?],
        )?;

        Ok(Self {
            _stream: stream,
            _listener: listener,
        })
    }
}

fn format_pod(format: &Format) -> Result<Vec<u8>> {
    let mut info = AudioInfoRaw::new();
    info.set_format(AudioFormat::F32LE);
    info.set_rate(format.rate);
    info.set_channels(format.positions.len() as u32);
    let mut position = [0; MAX_CHANNELS];
    position[..format.positions.len()].copy_from_slice(format.positions);
    info.set_position(position);

    let object = Object {
        type_: SpaTypes::ObjectParamFormat.as_raw(),
        id: ParamType::EnumFormat.as_raw(),
        properties: info.into(),
    };
    let (cursor, _) = PodSerializer::serialize(Cursor::new(Vec::new()), &Value::Object(object))
        .map_err(|err| anyhow!("serializing the format pod failed: {err:?}"))?;
    Ok(cursor.into_inner())
}
