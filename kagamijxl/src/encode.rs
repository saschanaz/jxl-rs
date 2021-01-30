use std::os::raw::c_int;

use libjxl_sys::*;

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

pub unsafe fn encode_oneshot(
    data: &[u8],
    xsize: usize,
    ysize: usize,
    enc: *mut JxlEncoderStruct,
    options: *mut JxlEncoderOptionsStruct,
) -> Result<Vec<u8>, &'static str> {
    let runner = JxlThreadParallelRunnerCreate(
        std::ptr::null(),
        JxlThreadParallelRunnerDefaultNumWorkerThreads(),
    );

    if JXL_ENC_SUCCESS != JxlEncoderSetParallelRunner(enc, Some(JxlThreadParallelRunner), runner) {
        JxlThreadParallelRunnerDestroy(runner);
        JxlEncoderDestroy(enc);
        return Err("JxlEncoderSetParallelRunner failed");
    }

    if JXL_ENC_SUCCESS != JxlEncoderSetDimensions(enc, xsize, ysize) {
        JxlThreadParallelRunnerDestroy(runner);
        JxlEncoderDestroy(enc);
        return Err("JxlEncoderSetDimensions failed");
    }

    let pixel_format = JxlPixelFormat {
        num_channels: 4,
        data_type: JXL_TYPE_UINT8,
        endianness: JXL_NATIVE_ENDIAN,
        align: 0,
    };

    if JXL_ENC_SUCCESS
        != JxlEncoderAddImageFrame(
            options, // moving ownership, no need to destroy later
            &pixel_format,
            data.as_ptr() as *mut std::ffi::c_void,
            data.len(),
        )
    {
        JxlThreadParallelRunnerDestroy(runner);
        JxlEncoderDestroy(enc);
        return Err("JxlEncoderAddImageFrame failed");
    }

    let result = encode_loop(enc);

    JxlThreadParallelRunnerDestroy(runner);
    JxlEncoderDestroy(enc);

    result
}

#[derive(Default)]
pub struct Encoder {
    pub dimensions: Option<(usize, usize)>,
    pub lossless: Option<bool>,
    pub effort: Option<i32>,
    pub distance: Option<f32>,
}

impl Encoder {
    #[inline]
    pub fn new() -> Self {
        Self::default()
    }

    unsafe fn create_options(&self, enc: *mut JxlEncoderStruct) -> *mut JxlEncoderOptionsStruct {
        let options = JxlEncoderOptionsCreate(enc, std::ptr::null());

        if let Some(lossless) = self.lossless {
            JxlEncoderOptionsSetLossless(options, lossless as i32);
        }
        if let Some(effort) = self.effort {
            JxlEncoderOptionsSetEffort(options, effort as c_int);
        }
        if let Some(distance) = self.distance {
            JxlEncoderOptionsSetDistance(options, distance);
        }

        options
    }

    pub fn encode(&self, data: &[u8], xsize: usize, ysize: usize) -> Result<Vec<u8>, &'static str> {
        unsafe {
            let enc = JxlEncoderCreate(std::ptr::null());
            if let Some(dimensions) = self.dimensions {
                JxlEncoderSetDimensions(enc, dimensions.0, dimensions.1);
            }
            encode_oneshot(data, xsize, ysize, enc, self.create_options(enc))
        }
    }
}
