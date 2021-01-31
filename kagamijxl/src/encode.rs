use std::{ffi::c_void, os::raw::c_int};

use libjxl_sys::*;

macro_rules! try_enc {
    ($left:expr) => {{
        if $left != JXL_ENC_SUCCESS {
            return Err("Encoder failed");
        }
    }};
}

unsafe fn encode_loop(enc: *mut JxlEncoderStruct) -> Result<Vec<u8>, &'static str> {
    let mut compressed: Vec<u8> = Vec::new();
    compressed.resize(64, 0);
    let mut next_out = compressed.as_mut_ptr();
    let mut avail_out = compressed.len();
    loop {
        let process_result = JxlEncoderProcessOutput(enc, &mut next_out, &mut avail_out);
        match process_result {
            JXL_ENC_NEED_MORE_OUTPUT => {
                let offset = next_out.offset_from(compressed.as_ptr());
                compressed.resize(compressed.len() * 2, 0);
                next_out = compressed.as_mut_ptr().offset(offset);
                avail_out = compressed.len() - offset as usize;
            }
            JXL_ENC_SUCCESS => {
                compressed.resize(compressed.len() - avail_out, 0);
                return Ok(compressed);
            }
            _ => return Err("JxlEncoderProcessOutput failed"),
        }
    }
}

unsafe fn prepare_encoder(
    enc: &Encoder,
    enc_raw: *mut JxlEncoderStruct,
    xsize: usize,
    ysize: usize,
    runner: *mut c_void,
) -> Result<*mut JxlEncoderOptionsStruct, &'static str> {
    try_enc!(JxlEncoderSetParallelRunner(
        enc_raw,
        Some(JxlThreadParallelRunner),
        runner
    ));
    try_enc!(JxlEncoderSetDimensions(enc_raw, xsize, ysize));

    Ok(enc.create_options(enc_raw)?)
}

pub unsafe fn encode_oneshot(
    data: &[u8],
    xsize: usize,
    ysize: usize,
    enc: &Encoder,
) -> Result<Vec<u8>, &'static str> {
    let runner = JxlThreadParallelRunnerCreate(
        std::ptr::null(),
        JxlThreadParallelRunnerDefaultNumWorkerThreads(),
    );

    let enc_raw = JxlEncoderCreate(std::ptr::null());

    let preparation = prepare_encoder(&enc, enc_raw, xsize, ysize, runner);
    if preparation.is_err() {
        JxlThreadParallelRunnerDestroy(runner);
        JxlEncoderDestroy(enc_raw);
        return Err("Couldn't prepare the encoder");
    }

    // Options struct is tied to the encoder and thus destructs together
    let options = preparation.unwrap();

    let pixel_format = JxlPixelFormat {
        num_channels: 4,
        data_type: JXL_TYPE_UINT8,
        endianness: JXL_NATIVE_ENDIAN,
        align: 0,
    };

    if JXL_ENC_SUCCESS
        != JxlEncoderAddImageFrame(
            options,
            &pixel_format,
            data.as_ptr() as *mut std::ffi::c_void,
            data.len(),
        )
    {
        JxlThreadParallelRunnerDestroy(runner);
        JxlEncoderDestroy(enc_raw);
        return Err("JxlEncoderAddImageFrame failed");
    }

    let result = encode_loop(enc_raw);

    JxlThreadParallelRunnerDestroy(runner);
    JxlEncoderDestroy(enc_raw);

    result
}

#[derive(Default)]
pub struct Encoder {
    // pub dimensions: Option<(usize, usize)>,
    pub lossless: Option<bool>,
    pub effort: Option<i32>,
    pub distance: Option<f32>,
}

impl Encoder {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    unsafe fn create_options(&self, enc: *mut JxlEncoderStruct) -> Result<*mut JxlEncoderOptionsStruct, &'static str> {
        let options = JxlEncoderOptionsCreate(enc, std::ptr::null());

        if let Some(lossless) = self.lossless {
            try_enc!(JxlEncoderOptionsSetLossless(options, lossless as i32));
        }
        if let Some(effort) = self.effort {
            try_enc!(JxlEncoderOptionsSetEffort(options, effort as c_int));
        }
        if let Some(distance) = self.distance {
            try_enc!(JxlEncoderOptionsSetDistance(options, distance));
        }

        Ok(options)
    }

    pub fn encode(&self, data: &[u8], xsize: usize, ysize: usize) -> Result<Vec<u8>, &'static str> {
        unsafe {
            encode_oneshot(data, xsize, ysize, &self)
        }
    }
}
