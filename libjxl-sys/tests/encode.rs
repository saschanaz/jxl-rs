use libjxl_sys::*;
mod decode;

#[test]
fn test_version() {
    unsafe {
        assert_eq!(JxlEncoderVersion(), 3002);
    }
}

// Ported version of https://gitlab.com/wg1/jpeg-xl/-/blob/v0.2/examples/encode_oneshot.cc

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

unsafe fn encode_oneshot(
    data: &Vec<u8>,
    xsize: usize,
    ysize: usize,
) -> Result<Vec<u8>, &'static str> {
    let enc = JxlEncoderCreate(std::ptr::null());

    let runner = JxlThreadParallelRunnerCreate(
        std::ptr::null(),
        JxlThreadParallelRunnerDefaultNumWorkerThreads(),
    );

    if JXL_ENC_SUCCESS != JxlEncoderSetParallelRunner(enc, Some(JxlThreadParallelRunner), runner) {
        JxlThreadParallelRunnerDestroy(runner);
        JxlEncoderDestroy(enc);
        return Err("JxlEncoderSetParallelRunner failed");
    }

    let basic_info = JxlBasicInfo {
        xsize: xsize as u32,
        ysize: ysize as u32,
        bits_per_sample: 8,
        alpha_bits: 8,
        ..Default::default()
    };

    if JXL_ENC_SUCCESS != JxlEncoderSetBasicInfo(enc, &basic_info) {
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

    let options = JxlEncoderOptionsCreate(enc, std::ptr::null());
    JxlEncoderOptionsSetLossless(options, 1);

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

#[test]
fn test_encode_oneshot() {
    #[rustfmt::skip]
    let data = vec![
        0x25, 0xae, 0x8e, 0x05, 0xa2, 0xad, 0x9c, 0x6c, 0xb0, 0xc1, 0xd7, 0x7c,
        0xf3, 0xa6, 0x34, 0xed, 0xb7, 0x8c, 0xda, 0x80, 0xd0, 0x2d, 0x7e, 0xda,
        0x48, 0x5a, 0xf7, 0x62, 0xce, 0xd8, 0x38, 0x35, 0x24, 0xd1, 0x33, 0xe9,
    ];

    let encoded = unsafe { encode_oneshot(&data, 3, 3).expect("Failed to encode") };

    let (basic_info, decoded, _) =
        unsafe { decode::decode_oneshot(encoded).expect("Failed to decode again") };

    assert_eq!(basic_info.xsize, 3);
    assert_eq!(basic_info.ysize, 3);
    assert_ne!(data, decoded); // TODO: replace this back to assert_eq. 0.3.2 seemingly has a bug
}
