use crate::{ConversionConfig, ConvertError, ConverterBackend, FrameConverter};
use ffmpeg::{format::Pixel, frame};
use parking_lot::Mutex;
use std::{
    ffi::c_void,
    ptr,
    sync::atomic::{AtomicBool, AtomicU64, Ordering},
};

type CFAllocatorRef = *const c_void;
type CFDictionaryRef = *const c_void;
type CVPixelBufferRef = *mut c_void;
type VTPixelTransferSessionRef = *mut c_void;
type OSStatus = i32;

const K_CV_RETURN_SUCCESS: i32 = 0;

const K_CV_PIXEL_FORMAT_TYPE_422_YP_CB_YP_CR8: u32 = 0x79757679;
const K_CV_PIXEL_FORMAT_TYPE_420_YP_CB_CR8_BI_PLANAR_VIDEO_RANGE: u32 = 0x34323076;
const K_CV_PIXEL_FORMAT_TYPE_2VUY: u32 = 0x32767579;
const K_CV_PIXEL_FORMAT_TYPE_32_BGRA: u32 = 0x42475241;
const K_CV_PIXEL_FORMAT_TYPE_32_ARGB: u32 = 0x00000020;
const K_CV_PIXEL_FORMAT_TYPE_32_RGBA: u32 = 0x52474241;

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFRelease(cf: *const c_void);
}

#[link(name = "CoreVideo", kind = "framework")]
unsafe extern "C" {
    fn CVPixelBufferCreate(
        allocator: CFAllocatorRef,
        width: usize,
        height: usize,
        pixel_format_type: u32,
        pixel_buffer_attributes: CFDictionaryRef,
        pixel_buffer_out: *mut CVPixelBufferRef,
    ) -> i32;

    fn CVPixelBufferCreateWithBytes(
        allocator: CFAllocatorRef,
        width: usize,
        height: usize,
        pixel_format_type: u32,
        base_address: *mut c_void,
        bytes_per_row: usize,
        release_callback: *const c_void,
        release_ref_con: *const c_void,
        pixel_buffer_attributes: CFDictionaryRef,
        pixel_buffer_out: *mut CVPixelBufferRef,
    ) -> i32;

    fn CVPixelBufferRelease(pixel_buffer: CVPixelBufferRef);

    fn CVPixelBufferLockBaseAddress(pixel_buffer: CVPixelBufferRef, lock_flags: u64) -> i32;
    fn CVPixelBufferUnlockBaseAddress(pixel_buffer: CVPixelBufferRef, lock_flags: u64) -> i32;

    fn CVPixelBufferGetBaseAddressOfPlane(pixel_buffer: CVPixelBufferRef, plane: usize) -> *mut u8;
    fn CVPixelBufferGetBytesPerRowOfPlane(pixel_buffer: CVPixelBufferRef, plane: usize) -> usize;
    fn CVPixelBufferGetHeightOfPlane(pixel_buffer: CVPixelBufferRef, plane: usize) -> usize;
    fn CVPixelBufferGetPlaneCount(pixel_buffer: CVPixelBufferRef) -> usize;
    fn CVPixelBufferGetBaseAddress(pixel_buffer: CVPixelBufferRef) -> *mut u8;
    fn CVPixelBufferGetBytesPerRow(pixel_buffer: CVPixelBufferRef) -> usize;
    fn CVPixelBufferGetHeight(pixel_buffer: CVPixelBufferRef) -> usize;
}

#[link(name = "VideoToolbox", kind = "framework")]
unsafe extern "C" {
    fn VTPixelTransferSessionCreate(
        allocator: CFAllocatorRef,
        pixel_transfer_session_out: *mut VTPixelTransferSessionRef,
    ) -> OSStatus;

    fn VTPixelTransferSessionInvalidate(session: VTPixelTransferSessionRef);

    fn VTPixelTransferSessionTransferImage(
        session: VTPixelTransferSessionRef,
        source_buffer: CVPixelBufferRef,
        destination_buffer: CVPixelBufferRef,
    ) -> OSStatus;
}

fn pixel_to_cv_format(pixel: Pixel) -> Option<u32> {
    match pixel {
        Pixel::YUYV422 => Some(K_CV_PIXEL_FORMAT_TYPE_422_YP_CB_YP_CR8),
        Pixel::UYVY422 => Some(K_CV_PIXEL_FORMAT_TYPE_2VUY),
        Pixel::NV12 => Some(K_CV_PIXEL_FORMAT_TYPE_420_YP_CB_CR8_BI_PLANAR_VIDEO_RANGE),
        Pixel::BGRA => Some(K_CV_PIXEL_FORMAT_TYPE_32_BGRA),
        Pixel::ARGB => Some(K_CV_PIXEL_FORMAT_TYPE_32_ARGB),
        _ => None,
    }
}

struct SessionHandle(VTPixelTransferSessionRef);

unsafe impl Send for SessionHandle {}

pub struct VideoToolboxConverter {
    session: Mutex<SessionHandle>,
    input_format: Pixel,
    input_cv_format: u32,
    output_format: Pixel,
    output_cv_format: u32,
    input_width: u32,
    input_height: u32,
    output_width: u32,
    output_height: u32,
    conversion_count: AtomicU64,
    verified_hardware: AtomicBool,
}

impl VideoToolboxConverter {
    pub fn new_rgba_to_nv12(width: u32, height: u32) -> Result<Self, ConvertError> {
        let input_cv_format = K_CV_PIXEL_FORMAT_TYPE_32_RGBA;
        let output_cv_format = K_CV_PIXEL_FORMAT_TYPE_420_YP_CB_CR8_BI_PLANAR_VIDEO_RANGE;

        let mut session: VTPixelTransferSessionRef = ptr::null_mut();
        let status = unsafe { VTPixelTransferSessionCreate(ptr::null(), &mut session) };

        if status != 0 {
            return Err(ConvertError::HardwareUnavailable(format!(
                "VTPixelTransferSessionCreate failed with status: {status}"
            )));
        }

        if session.is_null() {
            return Err(ConvertError::HardwareUnavailable(
                "VTPixelTransferSessionCreate returned null session".to_string(),
            ));
        }

        tracing::debug!(
            "[T002-S06] VideoToolbox converter initialized: RGBA {}x{} -> NV12",
            width,
            height
        );

        Ok(Self {
            session: Mutex::new(SessionHandle(session)),
            input_format: Pixel::RGBA,
            input_cv_format,
            output_format: Pixel::NV12,
            output_cv_format,
            input_width: width,
            input_height: height,
            output_width: width,
            output_height: height,
            conversion_count: AtomicU64::new(0),
            verified_hardware: AtomicBool::new(false),
        })
    }

    pub fn new(config: ConversionConfig) -> Result<Self, ConvertError> {
        let input_cv_format = pixel_to_cv_format(config.input_format).ok_or(
            ConvertError::UnsupportedFormat(config.input_format, config.output_format),
        )?;

        let output_cv_format = pixel_to_cv_format(config.output_format).ok_or(
            ConvertError::UnsupportedFormat(config.input_format, config.output_format),
        )?;

        let mut session: VTPixelTransferSessionRef = ptr::null_mut();
        let status = unsafe { VTPixelTransferSessionCreate(ptr::null(), &mut session) };

        if status != 0 {
            return Err(ConvertError::HardwareUnavailable(format!(
                "VTPixelTransferSessionCreate failed with status: {status}"
            )));
        }

        if session.is_null() {
            return Err(ConvertError::HardwareUnavailable(
                "VTPixelTransferSessionCreate returned null session".to_string(),
            ));
        }

        tracing::debug!(
            "VideoToolbox converter initialized: {:?} {}x{} -> {:?} {}x{}",
            config.input_format,
            config.input_width,
            config.input_height,
            config.output_format,
            config.output_width,
            config.output_height
        );

        Ok(Self {
            session: Mutex::new(SessionHandle(session)),
            input_format: config.input_format,
            input_cv_format,
            output_format: config.output_format,
            output_cv_format,
            input_width: config.input_width,
            input_height: config.input_height,
            output_width: config.output_width,
            output_height: config.output_height,
            conversion_count: AtomicU64::new(0),
            verified_hardware: AtomicBool::new(false),
        })
    }

    fn create_input_pixel_buffer(
        &self,
        input: &frame::Video,
    ) -> Result<CVPixelBufferRef, ConvertError> {
        let mut pixel_buffer: CVPixelBufferRef = ptr::null_mut();

        let base_address = input.data(0).as_ptr() as *mut c_void;
        let bytes_per_row = input.stride(0);

        let status = unsafe {
            CVPixelBufferCreateWithBytes(
                ptr::null(),
                self.input_width as usize,
                self.input_height as usize,
                self.input_cv_format,
                base_address,
                bytes_per_row,
                ptr::null(),
                ptr::null(),
                ptr::null(),
                &mut pixel_buffer,
            )
        };

        if status != K_CV_RETURN_SUCCESS {
            return Err(ConvertError::ConversionFailed(format!(
                "CVPixelBufferCreateWithBytes failed: {status}"
            )));
        }

        Ok(pixel_buffer)
    }

    fn create_output_pixel_buffer(&self) -> Result<CVPixelBufferRef, ConvertError> {
        let mut pixel_buffer: CVPixelBufferRef = ptr::null_mut();

        let status = unsafe {
            CVPixelBufferCreate(
                ptr::null(),
                self.output_width as usize,
                self.output_height as usize,
                self.output_cv_format,
                ptr::null(),
                &mut pixel_buffer,
            )
        };

        if status != K_CV_RETURN_SUCCESS {
            return Err(ConvertError::ConversionFailed(format!(
                "CVPixelBufferCreate failed: {status}"
            )));
        }

        Ok(pixel_buffer)
    }

    fn copy_output_to_frame(
        &self,
        pixel_buffer: CVPixelBufferRef,
        output: &mut frame::Video,
    ) -> Result<(), ConvertError> {
        unsafe {
            let lock_status = CVPixelBufferLockBaseAddress(pixel_buffer, 0);
            if lock_status != K_CV_RETURN_SUCCESS {
                return Err(ConvertError::ConversionFailed(format!(
                    "CVPixelBufferLockBaseAddress failed: {lock_status}"
                )));
            }
        }

        unsafe {
            let plane_count = CVPixelBufferGetPlaneCount(pixel_buffer);

            if plane_count == 0 {
                let src_ptr = CVPixelBufferGetBaseAddress(pixel_buffer);
                let src_stride = CVPixelBufferGetBytesPerRow(pixel_buffer);
                let height = CVPixelBufferGetHeight(pixel_buffer);
                let dst_stride = output.stride(0);

                let dst_data = output.data_mut(0);
                let dst_ptr = dst_data.as_mut_ptr();

                if src_stride == dst_stride {
                    ptr::copy_nonoverlapping(src_ptr, dst_ptr, height * src_stride);
                } else {
                    let copy_len = src_stride.min(dst_stride);
                    for row in 0..height {
                        let src_row = src_ptr.add(row * src_stride);
                        let dst_row = dst_ptr.add(row * dst_stride);
                        ptr::copy_nonoverlapping(src_row, dst_row, copy_len);
                    }
                }
            } else {
                for plane in 0..plane_count {
                    let src_ptr = CVPixelBufferGetBaseAddressOfPlane(pixel_buffer, plane);
                    let src_stride = CVPixelBufferGetBytesPerRowOfPlane(pixel_buffer, plane);
                    let height = CVPixelBufferGetHeightOfPlane(pixel_buffer, plane);
                    let dst_stride = output.stride(plane);

                    let dst_data = output.data_mut(plane);
                    let dst_ptr = dst_data.as_mut_ptr();

                    if src_stride == dst_stride {
                        ptr::copy_nonoverlapping(src_ptr, dst_ptr, height * src_stride);
                    } else {
                        let copy_len = src_stride.min(dst_stride);
                        for row in 0..height {
                            let src_row = src_ptr.add(row * src_stride);
                            let dst_row = dst_ptr.add(row * dst_stride);
                            ptr::copy_nonoverlapping(src_row, dst_row, copy_len);
                        }
                    }
                }
            }

            CVPixelBufferUnlockBaseAddress(pixel_buffer, 0);
        }

        Ok(())
    }

    fn extract_nv12_planes(
        &self,
        pixel_buffer: CVPixelBufferRef,
    ) -> Result<(Vec<u8>, Vec<u8>), ConvertError> {
        unsafe {
            let lock_status = CVPixelBufferLockBaseAddress(pixel_buffer, 0);
            if lock_status != K_CV_RETURN_SUCCESS {
                return Err(ConvertError::ConversionFailed(format!(
                    "CVPixelBufferLockBaseAddress failed: {lock_status}"
                )));
            }
        }

        let (y_plane, uv_plane) = unsafe {
            let plane_count = CVPixelBufferGetPlaneCount(pixel_buffer);
            if plane_count != 2 {
                CVPixelBufferUnlockBaseAddress(pixel_buffer, 0);
                return Err(ConvertError::ConversionFailed(format!(
                    "Expected 2 planes for NV12, got {plane_count}"
                )));
            }

            let y_ptr = CVPixelBufferGetBaseAddressOfPlane(pixel_buffer, 0);
            let y_stride = CVPixelBufferGetBytesPerRowOfPlane(pixel_buffer, 0);
            let y_height = CVPixelBufferGetHeightOfPlane(pixel_buffer, 0);

            let uv_ptr = CVPixelBufferGetBaseAddressOfPlane(pixel_buffer, 1);
            let uv_stride = CVPixelBufferGetBytesPerRowOfPlane(pixel_buffer, 1);
            let uv_height = CVPixelBufferGetHeightOfPlane(pixel_buffer, 1);

            let mut y_plane = Vec::with_capacity(self.output_width as usize * y_height);
            let mut uv_plane = Vec::with_capacity(self.output_width as usize * uv_height);

            for row in 0..y_height {
                let src_row = y_ptr.add(row * y_stride);
                let row_slice = std::slice::from_raw_parts(src_row, self.output_width as usize);
                y_plane.extend_from_slice(row_slice);
            }

            for row in 0..uv_height {
                let src_row = uv_ptr.add(row * uv_stride);
                let row_slice = std::slice::from_raw_parts(src_row, self.output_width as usize);
                uv_plane.extend_from_slice(row_slice);
            }

            CVPixelBufferUnlockBaseAddress(pixel_buffer, 0);

            (y_plane, uv_plane)
        };

        Ok((y_plane, uv_plane))
    }

    pub fn convert_raw_rgba_to_nv12(
        &self,
        rgba_data: &[u8],
        width: u32,
        height: u32,
        stride: usize,
    ) -> Result<(Vec<u8>, Vec<u8>), ConvertError> {
        let count = self.conversion_count.fetch_add(1, Ordering::Relaxed);

        if count == 0 {
            tracing::info!(
                "[T002-S06] VideoToolbox converter first frame: RGBA {}x{} -> NV12",
                width,
                height
            );
        }

        let mut pixel_buffer: CVPixelBufferRef = ptr::null_mut();

        let base_address = rgba_data.as_ptr() as *mut c_void;

        let status = unsafe {
            CVPixelBufferCreateWithBytes(
                ptr::null(),
                width as usize,
                height as usize,
                K_CV_PIXEL_FORMAT_TYPE_32_RGBA,
                base_address,
                stride,
                ptr::null(),
                ptr::null(),
                ptr::null(),
                &mut pixel_buffer,
            )
        };

        if status != K_CV_RETURN_SUCCESS {
            return Err(ConvertError::ConversionFailed(format!(
                "CVPixelBufferCreateWithBytes failed: {status}"
            )));
        }

        let output_buffer = self.create_output_pixel_buffer()?;

        let transfer_status = {
            let session_guard = self.session.lock();
            unsafe {
                VTPixelTransferSessionTransferImage(session_guard.0, pixel_buffer, output_buffer)
            }
        };

        unsafe {
            CVPixelBufferRelease(pixel_buffer);
        }

        if transfer_status != 0 {
            unsafe {
                CVPixelBufferRelease(output_buffer);
            }
            return Err(ConvertError::ConversionFailed(format!(
                "VTPixelTransferSessionTransferImage failed: {transfer_status}"
            )));
        }

        if !self.verified_hardware.swap(true, Ordering::Relaxed) {
            tracing::info!(
                "[T002-S06] VideoToolbox VTPixelTransferSession succeeded - hardware acceleration confirmed"
            );
        }

        let result = self.extract_nv12_planes(output_buffer);

        unsafe {
            CVPixelBufferRelease(output_buffer);
        }

        result
    }
}

impl Drop for VideoToolboxConverter {
    fn drop(&mut self) {
        let session = self.session.get_mut().0;
        if !session.is_null() {
            unsafe {
                VTPixelTransferSessionInvalidate(session);
                CFRelease(session as *const c_void);
            }
        }
    }
}

impl FrameConverter for VideoToolboxConverter {
    fn convert(&self, input: frame::Video) -> Result<frame::Video, ConvertError> {
        let mut output =
            frame::Video::new(self.output_format, self.output_width, self.output_height);
        self.convert_into(input, &mut output)?;
        Ok(output)
    }

    fn convert_into(
        &self,
        input: frame::Video,
        output: &mut frame::Video,
    ) -> Result<(), ConvertError> {
        let count = self.conversion_count.fetch_add(1, Ordering::Relaxed);

        if count == 0 {
            tracing::info!(
                "VideoToolbox converter first frame: {:?} -> {:?}",
                self.input_format,
                self.output_format
            );
        }

        let input_buffer = self.create_input_pixel_buffer(&input)?;
        let output_buffer = self.create_output_pixel_buffer()?;

        let status = {
            let session_guard = self.session.lock();
            unsafe {
                VTPixelTransferSessionTransferImage(session_guard.0, input_buffer, output_buffer)
            }
        };

        unsafe {
            CVPixelBufferRelease(input_buffer);
        }

        if status != 0 {
            unsafe {
                CVPixelBufferRelease(output_buffer);
            }
            return Err(ConvertError::ConversionFailed(format!(
                "VTPixelTransferSessionTransferImage failed: {status}"
            )));
        }

        if !self.verified_hardware.swap(true, Ordering::Relaxed) {
            tracing::info!(
                "VideoToolbox VTPixelTransferSession succeeded - hardware acceleration confirmed"
            );
        }

        self.copy_output_to_frame(output_buffer, output)?;
        output.set_pts(input.pts());

        unsafe {
            CVPixelBufferRelease(output_buffer);
        }

        Ok(())
    }

    fn name(&self) -> &'static str {
        "videotoolbox"
    }

    fn backend(&self) -> ConverterBackend {
        ConverterBackend::VideoToolbox
    }

    fn conversion_count(&self) -> u64 {
        self.conversion_count.load(Ordering::Relaxed)
    }

    fn verify_hardware_usage(&self) -> Option<bool> {
        Some(self.verified_hardware.load(Ordering::Relaxed))
    }
}
