use std::{
    ffi::c_void,
    fmt::Debug,
    fs::File,
    io::{BufRead, BufReader},
};

use crate::{contiguous_buffer::ContiguousBuffer, BasicInfo};
use libjxl_sys::*;

#[derive(Debug)]
pub enum JxlError {
    AllocationFailed,
    InputNotComplete,
    Fatal,
}

macro_rules! try_dec_fatal {
    ($left:expr) => {{
        if $left != JXL_DEC_SUCCESS {
            return Err(JxlError::Fatal);
        }
    }};
}

unsafe fn read_basic_info(
    dec: *mut JxlDecoderStruct,
    result: &mut DecodeProgress,
) -> Result<(), JxlError> {
    // Get the basic info
    try_dec_fatal!(JxlDecoderGetBasicInfo(dec, &mut result.basic_info));
    Ok(())
}

unsafe fn read_color_encoding(
    dec: *mut JxlDecoderStruct,
    result: &mut DecodeProgress,
    pixel_format: &JxlPixelFormat,
) -> Result<(), JxlError> {
    // Get the ICC color profile of the pixel data
    let mut icc_size = 0usize;
    try_dec_fatal!(JxlDecoderGetICCProfileSize(
        dec,
        pixel_format,
        JXL_COLOR_PROFILE_TARGET_DATA,
        &mut icc_size
    ));
    result.color_profile.resize(icc_size, 0);
    try_dec_fatal!(JxlDecoderGetColorAsICCProfile(
        dec,
        pixel_format,
        JXL_COLOR_PROFILE_TARGET_DATA,
        result.color_profile.as_mut_ptr(),
        icc_size
    ));
    Ok(())
}

unsafe fn prepare_frame(
    dec: *mut JxlDecoderStruct,
    result: &mut DecodeProgress,
) -> Result<(), JxlError> {
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
        name: String::from_utf8(name_vec).map_err(|_| JxlError::Fatal)?,
        duration: header.duration,
        timecode: header.timecode,
        is_last: header.is_last != 0,
        ..Default::default()
    };
    result.frames.push(frame);
    Ok(())
}

unsafe fn prepare_preview_out_buffer(
    dec: *mut JxlDecoderStruct,
    result: &mut DecodeProgress,
    pixel_format: &JxlPixelFormat,
) -> Result<(), JxlError> {
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

unsafe fn prepare_image_out_buffer(
    dec: *mut JxlDecoderStruct,
    result: &mut DecodeProgress,
    pixel_format: &JxlPixelFormat,
) -> Result<(), JxlError> {
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

unsafe fn decode_loop(
    progress: &mut DecodeProgress,
    data: impl BufRead,
    pixel_format: &JxlPixelFormat,
    event_flags: JxlDecoderStatus,
    max_frames: Option<usize>,
    allow_partial: bool,
) -> Result<(), JxlError> {
    let dec = progress.raw.decoder;

    try_dec_fatal!(JxlDecoderSubscribeEvents(dec, event_flags as i32));

    let mut buffer = ContiguousBuffer::new(data);
    try_dec_fatal!(JxlDecoderSetInput(dec, buffer.as_ptr(), buffer.len()));

    loop {
        let status = JxlDecoderProcessInput(dec);

        match status {
            JXL_DEC_ERROR => return Err(JxlError::Fatal),
            JXL_DEC_NEED_MORE_INPUT => {
                let remaining = JxlDecoderReleaseInput(dec);
                let consumed = buffer.len() - remaining;
                buffer.consume(consumed);

                if buffer.more_buf().is_err() {
                    if allow_partial {
                        progress.is_partial = true;
                        try_dec_fatal!(JxlDecoderFlushImage(dec));
                        break;
                    } else {
                        return Err(JxlError::InputNotComplete);
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
                // Nothing to do. Do not yet return. If the image is an animation, more
                // full frames may be decoded.
                if max_frames.is_some() && progress.frames.len() == max_frames.unwrap() {
                    break;
                }
            }
            JXL_DEC_SUCCESS => {
                // All decoding successfully finished.
                break;
            }
            _ => return Err(JxlError::Fatal), // Unknown status
        }
    }

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
        flags |= JXL_DEC_FRAME | JXL_DEC_FULL_IMAGE;
    }
    flags
}

fn prepare_decoder(
    keep_orientation: Option<bool>,
    dec_raw: *mut JxlDecoderStruct,
    runner: *mut c_void,
) -> Result<(), JxlError> {
    if let Some(keep_orientation) = keep_orientation {
        try_dec_fatal!(unsafe { JxlDecoderSetKeepOrientation(dec_raw, keep_orientation as i32) });
    }
    try_dec_fatal!(unsafe {
        JxlDecoderSetParallelRunner(dec_raw, Some(JxlThreadParallelRunner), runner)
    });
    Ok(())
}

pub unsafe fn decode_oneshot(
    data: impl BufRead,
    dec: &Decoder,
) -> Result<DecodeProgress, JxlError> {
    let mut progress = DecodeProgress::new(dec.keep_orientation)?;

    let event_flags = get_event_subscription_flags(dec);
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
        event_flags,
        dec.max_frames,
        dec.allow_partial,
    )?;

    Ok(progress)
}

#[derive(Default)]
pub struct Decoder {
    pub keep_orientation: Option<bool>,

    // pub pixel_format: Option<JxlPixelFormat>,
    pub need_color_profile: bool,
    pub need_optional_preview: bool,
    pub no_full_frame: bool,

    /** Specify when you need at most N frames */
    pub max_frames: Option<usize>,
    /** Specify when partial input is expected */
    pub allow_partial: bool,
}

impl Decoder {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn decode(&self, data: &[u8]) -> Result<DecodeProgress, JxlError> {
        // Just a helpful alias of decode_buffer for Vec which doesn't implement BufRead by itself
        self.decode_buffer(data)
    }

    pub fn decode_file(&self, file: &File) -> Result<DecodeProgress, JxlError> {
        self.decode_buffer(BufReader::new(file))
    }

    pub fn decode_buffer(&self, buffer: impl BufRead) -> Result<DecodeProgress, JxlError> {
        unsafe { decode_oneshot(buffer, self) }
    }

    // TODO:
    // Each decode call creates a new JxlDecoder but that doesn't need to be the case.
    // How about a static decode() receiving an option bag and returning a ongoing decode object including a live JxlDecoder, so that something like .proceed() can continue the decode?
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

    is_partial: bool,

    pub basic_info: BasicInfo,
    /** Can be empty unless `need_color_profile` is specified */
    pub color_profile: Vec<u8>,
    /** Can be empty unless `need_optional_preview` is specified */
    pub preview: Vec<u8>,
    /** Can be empty if neither of `need_frame_header` nor `need_frame` is specified */
    pub frames: Vec<Frame>,
}

impl DecodeProgress {
    pub fn new(keep_orientation: Option<bool>) -> Result<DecodeProgress, JxlError> {
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

            is_partial: false,

            basic_info: BasicInfo::default(),
            color_profile: Vec::new(),
            preview: Vec::new(),
            frames: Vec::new(),
        })
    }

    pub fn is_partial(&self) -> bool {
        self.is_partial
    }

    pub fn proceed(&self) {}
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
