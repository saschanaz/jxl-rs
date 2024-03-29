use std::usize;

use libjxl_sys::*;

#[test]
fn test_version() {
    unsafe {
        assert_eq!(JxlDecoderVersion(), 7000);
    }
}

macro_rules! try_dec {
    ($left:expr) => {{
        if $left != JXL_DEC_SUCCESS {
            return Err("Decoder failed");
        }
    }};
}

// Ported version of https://gitlab.com/wg1/jpeg-xl/-/blob/v0.2/examples/decode_oneshot.cc

// Copyright (c) the JPEG XL Project
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

unsafe fn decode_loop(
    dec: *mut JxlDecoderStruct,
    data: Vec<u8>,
) -> Result<(JxlBasicInfo, Vec<u8>, Vec<u8>), &'static str> {
    let pixel_format = JxlPixelFormat {
        num_channels: 4,
        data_type: JXL_TYPE_UINT8,
        endianness: JXL_NATIVE_ENDIAN,
        align: 0,
    };

    let mut basic_info = JxlBasicInfo::default();
    let mut pixels_buffer: Vec<u8> = Vec::new();
    let mut icc_profile: Vec<u8> = Vec::new();

    try_dec!(JxlDecoderSubscribeEvents(
        dec,
        (JXL_DEC_BASIC_INFO | JXL_DEC_COLOR_ENCODING | JXL_DEC_FULL_IMAGE) as i32
    ));

    JxlDecoderSetInput(dec, data.as_ptr(), data.len());

    loop {
        let status = JxlDecoderProcessInput(dec);

        match status {
            JXL_DEC_ERROR => return Err("Decoder error"),
            JXL_DEC_NEED_MORE_INPUT => return Err("Error, already provided all input"),

            // Get the basic info
            JXL_DEC_BASIC_INFO => {
                try_dec!(JxlDecoderGetBasicInfo(dec, &mut basic_info));
            }

            JXL_DEC_COLOR_ENCODING => {
                // Get the ICC color profile of the pixel data
                let mut icc_size = 0usize;
                try_dec!(JxlDecoderGetICCProfileSize(
                    dec,
                    &pixel_format,
                    JXL_COLOR_PROFILE_TARGET_DATA,
                    &mut icc_size
                ));
                icc_profile.resize(icc_size, 0);
                try_dec!(JxlDecoderGetColorAsICCProfile(
                    dec,
                    &pixel_format,
                    JXL_COLOR_PROFILE_TARGET_DATA,
                    icc_profile.as_mut_ptr(),
                    icc_size
                ));
            }

            // Get the output buffer
            JXL_DEC_NEED_IMAGE_OUT_BUFFER => {
                let mut buffer_size = 0usize;
                try_dec!(JxlDecoderImageOutBufferSize(
                    dec,
                    &pixel_format,
                    &mut buffer_size
                ));

                if buffer_size != (basic_info.xsize * basic_info.ysize * 4) as usize {
                    return Err("Invalid out buffer size");
                }

                pixels_buffer.resize(buffer_size as usize, 0);
                try_dec!(JxlDecoderSetImageOutBuffer(
                    dec,
                    &pixel_format,
                    pixels_buffer.as_mut_ptr() as *mut std::ffi::c_void,
                    buffer_size,
                ));
            }

            JXL_DEC_FULL_IMAGE => {
                // Nothing to do. Do not yet return. If the image is an animation, more
                // full frames may be decoded. This example only keeps the last one.
                continue;
            }
            JXL_DEC_SUCCESS => {
                // All decoding successfully finished.
                return Ok((basic_info, pixels_buffer, icc_profile));
            }
            _ => return Err("Unknown decoder status"),
        }
    }
}

pub unsafe fn decode_oneshot(
    data: Vec<u8>,
) -> Result<(JxlBasicInfo, Vec<u8>, Vec<u8>), &'static str> {
    // Multi-threaded parallel runner.
    let runner = JxlThreadParallelRunnerCreate(
        std::ptr::null(),
        JxlThreadParallelRunnerDefaultNumWorkerThreads(),
    );

    let dec = JxlDecoderCreate(std::ptr::null());

    if JXL_DEC_SUCCESS != JxlDecoderSetParallelRunner(dec, Some(JxlThreadParallelRunner), runner) {
        JxlThreadParallelRunnerDestroy(runner);
        JxlDecoderDestroy(dec);
        return Err("JxlDecoderSubscribeEvents failed");
    }

    let result = decode_loop(dec, data);

    JxlThreadParallelRunnerDestroy(runner);
    JxlDecoderDestroy(dec);

    result
}

#[test]
fn test_decode_oneshot() {
    // Resolve path manually or it will fail when running each test
    let sample_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/sample.jxl");

    let data = std::fs::read(sample_path).expect("Failed to read the sample image");
    let (basic_info, _, _) =
        unsafe { decode_oneshot(data).expect("Failed to decode the sample image") };

    assert_eq!(basic_info.xsize, 1404);
    assert_eq!(basic_info.ysize, 936);
    assert_eq!(basic_info.have_container, 0);
    assert_eq!(basic_info.orientation, JXL_ORIENT_IDENTITY);
}
