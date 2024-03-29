use std::{
    ffi::c_void,
    fmt::Debug,
    fs::File,
    io::{BufRead, BufReader},
};

use crate::{contiguous_buffer::ContiguousBuffer, BasicInfo};
use libjxl_sys::*;

#[derive(Debug)]
pub enum JxlDecodeError {
    AllocationFailed,
    InputNotComplete,
    AlreadyFinished,
    General,
}

macro_rules! try_dec_fatal {
    ($left:expr) => {{
        if unsafe { $left } != JXL_DEC_SUCCESS {
            panic!("A fatal error occurred in kagamijxl::Decoder");
        }
    }};
}

fn read_basic_info(
    dec: *mut JxlDecoderStruct,
    result: &mut DecodeProgress,
) -> Result<(), JxlDecodeError> {
    // Get the basic info
    try_dec_fatal!(JxlDecoderGetBasicInfo(dec, &mut result.basic_info));
    Ok(())
}

fn read_color_encoding(
    dec: *mut JxlDecoderStruct,
    result: &mut DecodeProgress,
    pixel_format: &JxlPixelFormat,
) -> Result<(), JxlDecodeError> {
    // Get the ICC color profile of the pixel data
    let mut icc_size = 0usize;
    try_dec_fatal!(JxlDecoderGetICCProfileSize(
        dec,
        pixel_format,
        JXL_COLOR_PROFILE_TARGET_DATA,
        &mut icc_size,
    ));
    result.color_profile.resize(icc_size, 0);
    try_dec_fatal!(JxlDecoderGetColorAsICCProfile(
        dec,
        pixel_format,
        JXL_COLOR_PROFILE_TARGET_DATA,
        result.color_profile.as_mut_ptr(),
        icc_size,
    ));
    Ok(())
}

fn prepare_frame(
    dec: *mut JxlDecoderStruct,
    result: &mut DecodeProgress,
) -> Result<(), JxlDecodeError> {
    let mut header = JxlFrameHeader::default();
    try_dec_fatal!(JxlDecoderGetFrameHeader(dec, &mut header));

    let mut name_vec: Vec<u8> = Vec::new();
    name_vec.resize((header.name_length + 1) as usize, 0);
    try_dec_fatal!(JxlDecoderGetFrameName(
        dec,
        name_vec.as_mut_ptr() as *mut _,
        name_vec.len()
    ));

    name_vec.pop(); // The string ends with null which is redundant in Rust

    let frame = Frame {
        name: String::from_utf8_lossy(&name_vec[..]).to_string(),
        duration: header.duration,
        timecode: header.timecode,
        is_last: header.is_last != 0,
        ..Default::default()
    };
    result.frames.push(frame);
    Ok(())
}

fn prepare_preview_out_buffer(
    dec: *mut JxlDecoderStruct,
    result: &mut DecodeProgress,
    pixel_format: &JxlPixelFormat,
) -> Result<(), JxlDecodeError> {
    let mut buffer_size = 0usize;
    try_dec_fatal!(JxlDecoderPreviewOutBufferSize(
        dec,
        pixel_format,
        &mut buffer_size
    ));

    assert_eq!(
        buffer_size,
        (result.basic_info.xsize * result.basic_info.ysize * 4) as usize
    );

    let buffer = &mut result.preview;

    buffer.resize(buffer_size as usize, 0);
    try_dec_fatal!(JxlDecoderSetPreviewOutBuffer(
        dec,
        pixel_format,
        buffer.as_mut_ptr() as *mut _,
        buffer_size,
    ));
    Ok(())
}

fn prepare_image_out_buffer(
    dec: *mut JxlDecoderStruct,
    result: &mut DecodeProgress,
    pixel_format: &JxlPixelFormat,
) -> Result<(), JxlDecodeError> {
    let mut buffer_size = 0usize;
    try_dec_fatal!(JxlDecoderImageOutBufferSize(
        dec,
        pixel_format,
        &mut buffer_size
    ));

    assert_eq!(
        buffer_size,
        (result.basic_info.xsize * result.basic_info.ysize * 4) as usize
    );

    let buffer = &mut result
        .frames
        .last_mut()
        .expect("Frames vector is unexpectedly empty")
        .data;

    buffer.resize(buffer_size as usize, 0);
    try_dec_fatal!(JxlDecoderSetImageOutBuffer(
        dec,
        pixel_format,
        buffer.as_mut_ptr() as *mut _,
        buffer_size,
    ));
    Ok(())
}

fn decode_loop(
    progress: &mut DecodeProgress,
    data: impl BufRead,
    pixel_format: &JxlPixelFormat,
    stop_on_frame: bool,
    allow_partial: bool,
) -> Result<(), JxlDecodeError> {
    let dec = progress.raw.decoder;

    let mut buffer = ContiguousBuffer::new(progress.unread_buffer.take().unwrap_or_default(), data);

    try_dec_fatal!(JxlDecoderSetInput(dec, buffer.as_ptr(), buffer.len()));

    loop {
        let status = unsafe { JxlDecoderProcessInput(dec) };

        match status {
            JXL_DEC_NEED_MORE_INPUT => {
                let remaining = unsafe { JxlDecoderReleaseInput(dec) };
                let consumed = buffer.len() - remaining;
                buffer.consume(consumed);

                if buffer.more_buf().is_err() {
                    if allow_partial {
                        break;
                    } else {
                        return Err(JxlDecodeError::InputNotComplete);
                    }
                }

                try_dec_fatal!(JxlDecoderSetInput(dec, buffer.as_ptr(), buffer.len()));
            }

            JXL_DEC_BASIC_INFO => read_basic_info(dec, progress)?,

            JXL_DEC_COLOR_ENCODING => read_color_encoding(dec, progress, pixel_format)?,

            JXL_DEC_FRAME => prepare_frame(dec, progress)?,

            JXL_DEC_NEED_PREVIEW_OUT_BUFFER => {
                prepare_preview_out_buffer(dec, progress, pixel_format)?
            }

            // Get the output buffer
            JXL_DEC_NEED_IMAGE_OUT_BUFFER => prepare_image_out_buffer(dec, progress, pixel_format)?,

            JXL_DEC_FULL_IMAGE => {
                if stop_on_frame && !progress.frames.last().unwrap().is_last {
                    let remaining = unsafe { JxlDecoderReleaseInput(dec) };
                    let consumed = buffer.len() - remaining;
                    buffer.consume(consumed);
                    break;
                }
            }
            JXL_DEC_SUCCESS => {
                // All decoding successfully finished.
                progress.is_partial = false;
                break;
            }

            JXL_DEC_ERROR => return Err(JxlDecodeError::General),
            _ => panic!("Unexpected JXL decoding status found: {}", status),
        }
    }

    progress.unread_buffer = Some(buffer.take_unread());

    Ok(())
}

fn get_event_subscription_flags(dec: &Decoder) -> JxlDecoderStatus {
    let mut flags: JxlDecoderStatus = JXL_DEC_BASIC_INFO;
    if dec.need_color_profile {
        flags |= JXL_DEC_COLOR_ENCODING;
    }
    if dec.need_optional_preview {
        flags |= JXL_DEC_PREVIEW_IMAGE;
    }
    if !dec.no_full_frame {
        flags |= JXL_DEC_FRAME;
    }
    if !dec.no_full_image && !dec.no_full_frame {
        flags |= JXL_DEC_FULL_IMAGE;
    }
    flags
}

fn prepare_decoder(
    keep_orientation: Option<bool>,
    dec_raw: *mut JxlDecoderStruct,
    runner: *mut c_void,
) -> Result<(), JxlDecodeError> {
    if let Some(keep_orientation) = keep_orientation {
        try_dec_fatal!(JxlDecoderSetKeepOrientation(
            dec_raw,
            keep_orientation as i32
        ));
    }
    try_dec_fatal!(JxlDecoderSetParallelRunner(
        dec_raw,
        Some(JxlThreadParallelRunner),
        runner
    ));
    Ok(())
}

pub fn decode_oneshot(data: impl BufRead, dec: &Decoder) -> Result<DecodeProgress, JxlDecodeError> {
    let mut progress = DecodeProgress::new(dec.keep_orientation)?;

    let event_flags = get_event_subscription_flags(dec);
    try_dec_fatal!(JxlDecoderSubscribeEvents(
        progress.raw.decoder,
        event_flags as i32
    ));

    // TODO: Support different pixel format
    // Not sure how to type the output vector properly
    let pixel_format = JxlPixelFormat {
        num_channels: 4,
        data_type: JXL_TYPE_UINT8,
        endianness: JXL_NATIVE_ENDIAN,
        align: 0,
    };
    decode_loop(
        &mut progress,
        data,
        &pixel_format,
        dec.stop_on_frame,
        dec.allow_partial,
    )?;

    Ok(progress)
}

#[derive(Default)]
pub struct Decoder {
    pub keep_orientation: Option<bool>,

    // pub pixel_format: Option<JxlPixelFormat>,
    /** Reads color profile into `DecodeProgres::color_profile` when set to true */
    pub need_color_profile: bool,
    /** Tries reading preview image into `DecodeProgress::preview` when set to true */
    pub need_optional_preview: bool,
    /** Prevents reading frames at all when set to true, which means the length of DecodeProgress::frames becomes 0 */
    pub no_full_frame: bool,
    /** Reads frame header without pixels when set to true */
    pub no_full_image: bool,

    /** Specify if you want to stop on the first frame decode */
    pub stop_on_frame: bool,
    /** Specify when partial input is expected */
    pub allow_partial: bool,
}

impl Decoder {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn decode(&self, data: &[u8]) -> Result<DecodeProgress, JxlDecodeError> {
        // Just a helpful alias of decode_buffer for Vec which doesn't implement BufRead by itself
        self.decode_buffer(data)
    }

    pub fn decode_file(&self, file: &File) -> Result<DecodeProgress, JxlDecodeError> {
        self.decode_buffer(BufReader::new(file))
    }

    pub fn decode_buffer(&self, buffer: impl BufRead) -> Result<DecodeProgress, JxlDecodeError> {
        decode_oneshot(buffer, self)
    }
}

struct DecodeRaw {
    decoder: *mut JxlDecoderStruct,
    parallel_runner: *mut c_void,
}

impl Drop for DecodeRaw {
    fn drop(&mut self) {
        unsafe {
            JxlThreadParallelRunnerDestroy(self.parallel_runner);
            JxlDecoderDestroy(self.decoder);
        }
    }
}

pub struct DecodeProgress {
    raw: DecodeRaw,
    unread_buffer: Option<Vec<u8>>,

    is_partial: bool,

    pub basic_info: BasicInfo,
    /** Can be empty unless `need_color_profile` is specified */
    pub color_profile: Vec<u8>,
    /** Can be empty unless `need_optional_preview` is specified */
    pub preview: Vec<u8>,
    /** Can be empty if neither of `need_frame_header` nor `need_frame` is specified */
    pub frames: Vec<Frame>,
}

impl Debug for DecodeProgress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DecodeProgress")
            .field("is_partial", &self.is_partial)
            .field("basic_info", &self.basic_info)
            .finish()
    }
}

impl DecodeProgress {
    pub fn new(keep_orientation: Option<bool>) -> Result<DecodeProgress, JxlDecodeError> {
        let decoder = unsafe { JxlDecoderCreate(std::ptr::null()) };
        let parallel_runner = unsafe {
            JxlThreadParallelRunnerCreate(
                std::ptr::null(),
                JxlThreadParallelRunnerDefaultNumWorkerThreads(),
            )
        };

        prepare_decoder(keep_orientation, decoder, parallel_runner)?;

        Ok(DecodeProgress {
            raw: DecodeRaw {
                decoder,
                parallel_runner,
            },
            unread_buffer: None,

            is_partial: true,

            basic_info: BasicInfo::default(),
            color_profile: Vec::new(),
            preview: Vec::new(),
            frames: Vec::new(),
        })
    }

    pub fn is_partial(&self) -> bool {
        self.is_partial
    }

    pub fn proceed(
        &mut self,
        data: impl BufRead,
        allow_partial: bool,
        stop_on_frame: bool,
    ) -> Result<(), JxlDecodeError> {
        if !self.is_partial {
            return Err(JxlDecodeError::AlreadyFinished);
        }

        // TODO: Support different pixel format
        // Not sure how to type the output vector properly
        let pixel_format = JxlPixelFormat {
            num_channels: 4,
            data_type: JXL_TYPE_UINT8,
            endianness: JXL_NATIVE_ENDIAN,
            align: 0,
        };
        decode_loop(self, data, &pixel_format, stop_on_frame, allow_partial)?;
        Ok(())
    }

    pub fn flush(&mut self) {
        unsafe { JxlDecoderFlushImage(self.raw.decoder) };
    }
}

#[derive(Default)]
pub struct Frame {
    pub name: String,
    pub duration: u32,
    pub timecode: u32,
    pub is_last: bool,

    /** Can be empty when `no_full_frame` is specified */
    pub data: Vec<u8>,
}
