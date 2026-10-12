use crate::{error::CaptureError, Frame, PixelFormat, ScreenCapture};
use std::time::{SystemTime, UNIX_EPOCH};
use windows::core::ComInterface;
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D, D3D11_CPU_ACCESS_READ,
    D3D11_CREATE_DEVICE_FLAG, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ, D3D11_SDK_VERSION,
    D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
};
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT_B8G8R8A8_UNORM;
use windows::Win32::Graphics::Dxgi::{
    IDXGIAdapter, IDXGIDevice, IDXGIOutput, IDXGIOutput1, IDXGIOutputDuplication, IDXGIResource,
    DXGI_OUTDUPL_DESC, DXGI_OUTDUPL_FRAME_INFO,
};

pub struct WindowsScreenCapture {
    duplication: IDXGIOutputDuplication,
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    width: u32,
    height: u32,
    output_index: u32,
    /// Reusable CPU-readable staging texture, (re)created when the frame size changes.
    staging: Option<ID3D11Texture2D>,
}

impl WindowsScreenCapture {
    pub fn new(_adapter_index: u32, output_index: u32) -> Result<Self, CaptureError> {
        unsafe {
            let mut device = None;
            let mut context = None;
            let feature_levels = [D3D_FEATURE_LEVEL_11_0];

            // 1. Create D3D11 Device
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                None,
                D3D11_CREATE_DEVICE_FLAG(0),
                Some(&feature_levels),
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )
            .map_err(|e| CaptureError::InitFailed(e.to_string()))?;

            let device = device.unwrap();
            let context = context.unwrap();

            // 2. Get DXGI Device, Adapter
            let dxgi_device: IDXGIDevice = device
                .cast()
                .map_err(|e| CaptureError::InitFailed(e.to_string()))?;
            let dxgi_adapter: IDXGIAdapter = dxgi_device
                .GetAdapter()
                .map_err(|e| CaptureError::InitFailed(e.to_string()))?;

            // 3. Get Specific Output (Monitor)
            let dxgi_output: IDXGIOutput = dxgi_adapter
                .EnumOutputs(output_index)
                .map_err(|e| CaptureError::InitFailed(e.to_string()))?;
            let dxgi_output1: IDXGIOutput1 = dxgi_output
                .cast()
                .map_err(|e| CaptureError::InitFailed(e.to_string()))?;

            // 4. Duplicate Output
            let duplication = dxgi_output1
                .DuplicateOutput(&device)
                .map_err(|e| CaptureError::InitFailed(e.to_string()))?;

            // 5. Parse the real resolution from the duplication descriptor.
            let mut desc = DXGI_OUTDUPL_DESC::default();
            duplication.GetDesc(&mut desc);
            let width = desc.ModeDesc.Width;
            let height = desc.ModeDesc.Height;

            Ok(Self {
                duplication,
                device,
                context,
                width,
                height,
                output_index,
                staging: None,
            })
        }
    }

    pub fn list_monitors() -> Vec<String> {
        let mut monitors = Vec::new();
        unsafe {
            let mut device = None;
            let mut context = None;
            let feature_levels = [D3D_FEATURE_LEVEL_11_0];

            if D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                None,
                D3D11_CREATE_DEVICE_FLAG(0),
                Some(&feature_levels),
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )
            .is_ok()
            {
                if let Some(device) = device {
                    if let Ok(dxgi_device) = device.cast::<IDXGIDevice>() {
                        if let Ok(dxgi_adapter) = dxgi_device.GetAdapter() {
                            let mut i = 0;
                            while let Ok(output) = dxgi_adapter.EnumOutputs(i) {
                                let mut desc =
                                    windows::Win32::Graphics::Dxgi::DXGI_OUTPUT_DESC::default();
                                if output.GetDesc(&mut desc).is_ok() {
                                    let name_len = desc
                                        .DeviceName
                                        .iter()
                                        .position(|&c| c == 0)
                                        .unwrap_or(desc.DeviceName.len());
                                    let name_utf16 = &desc.DeviceName[..name_len];
                                    let name = String::from_utf16_lossy(name_utf16);
                                    monitors.push(format!("Monitor {}: {}", i, name));
                                }
                                i += 1;
                            }
                        }
                    }
                }
            }
        }
        monitors
    }

    /// Ensure the staging texture exists and matches `desc` dimensions.
    /// The staging texture is CPU-readable so we can Map it and read pixels.
    unsafe fn ensure_staging(&mut self, src_desc: &D3D11_TEXTURE2D_DESC) -> Result<(), CaptureError> {
        let needs_new = match &self.staging {
            None => true,
            Some(tex) => {
                let mut existing = D3D11_TEXTURE2D_DESC::default();
                tex.GetDesc(&mut existing);
                existing.Width != src_desc.Width || existing.Height != src_desc.Height
            }
        };

        if needs_new {
            let staging_desc = D3D11_TEXTURE2D_DESC {
                Width: src_desc.Width,
                Height: src_desc.Height,
                MipLevels: 1,
                ArraySize: 1,
                Format: src_desc.Format,
                SampleDesc: src_desc.SampleDesc,
                Usage: D3D11_USAGE_STAGING,
                BindFlags: 0,
                CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                MiscFlags: 0,
            };
            let mut staging: Option<ID3D11Texture2D> = None;
            self.device
                .CreateTexture2D(&staging_desc, None, Some(&mut staging))
                .map_err(|e| CaptureError::PlatformError(e.to_string()))?;
            self.staging = staging;
        }
        Ok(())
    }
}

impl ScreenCapture for WindowsScreenCapture {
    fn capture_frame(&mut self) -> Result<Frame, CaptureError> {
        unsafe {
            let mut frame_info = DXGI_OUTDUPL_FRAME_INFO::default();
            let mut resource: Option<IDXGIResource> = None;

            match self
                .duplication
                .AcquireNextFrame(250, &mut frame_info, &mut resource)
            {
                Ok(_) => {
                    // Wrap the acquired frame so we always release it, even on early return.
                    let result = (|| {
                        let resource = resource
                            .ok_or_else(|| CaptureError::PlatformError("Null frame resource".into()))?;

                        // The acquired resource is a GPU texture we cannot read from the CPU.
                        let src_texture: ID3D11Texture2D = resource
                            .cast()
                            .map_err(|e| CaptureError::PlatformError(e.to_string()))?;

                        let mut src_desc = D3D11_TEXTURE2D_DESC::default();
                        src_texture.GetDesc(&mut src_desc);

                        // Keep our tracked resolution in sync with reality.
                        self.width = src_desc.Width;
                        self.height = src_desc.Height;

                        // Ensure a CPU-readable staging texture of the same size exists,
                        // then copy GPU -> staging.
                        self.ensure_staging(&src_desc)?;
                        let staging = self
                            .staging
                            .as_ref()
                            .ok_or_else(|| CaptureError::PlatformError("No staging texture".into()))?;
                        self.context.CopyResource(staging, &src_texture);

                        // Map the staging texture into CPU-visible memory.
                        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
                        self.context
                            .Map(staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))
                            .map_err(|e| CaptureError::PlatformError(e.to_string()))?;

                        let width = src_desc.Width as usize;
                        let height = src_desc.Height as usize;
                        let row_pitch = mapped.RowPitch as usize;
                        let tight_row = width * 4; // BGRA8 = 4 bytes/px

                        // Copy row-by-row to strip the GPU row padding (RowPitch >= tight_row).
                        let mut data = vec![0u8; tight_row * height];
                        let src_base = mapped.pData as *const u8;
                        for y in 0..height {
                            let src_row = src_base.add(y * row_pitch);
                            let dst_off = y * tight_row;
                            std::ptr::copy_nonoverlapping(
                                src_row,
                                data.as_mut_ptr().add(dst_off),
                                tight_row,
                            );
                        }

                        self.context.Unmap(staging, 0);

                        let pixel_format = if src_desc.Format == DXGI_FORMAT_B8G8R8A8_UNORM {
                            PixelFormat::Bgra8
                        } else {
                            PixelFormat::Unknown
                        };

                        Ok(Frame {
                            data,
                            width: src_desc.Width,
                            height: src_desc.Height,
                            pixel_format,
                            timestamp_ms: SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .unwrap()
                                .as_millis() as u64,
                            monitor_id: self.output_index,
                        })
                    })();

                    // Always release the DXGI frame before returning.
                    let _ = self.duplication.ReleaseFrame();
                    result
                }
                Err(e) => {
                    if e.code().0 == 0x887A0027u32 as i32 {
                        // DXGI_ERROR_WAIT_TIMEOUT
                        Err(CaptureError::Timeout)
                    } else if e.code().0 == 0x887A0026u32 as i32 {
                        // DXGI_ERROR_ACCESS_LOST
                        Err(CaptureError::DisplayChanged)
                    } else {
                        Err(CaptureError::PlatformError(e.to_string()))
                    }
                }
            }
        }
    }

    fn resolution(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn display_changed(&self) -> bool {
        false
    }

    fn switch_monitor(&mut self, id: u32) -> Result<(), CaptureError> {
        // No-op if already on this monitor.
        if id == self.output_index {
            return Ok(());
        }
        // Validate the target exists before tearing anything down, so a bad
        // index (e.g. monitor 1 on a single-monitor machine) can't corrupt the
        // currently-working capture.
        let count = Self::list_monitors().len() as u32;
        if id >= count {
            return Err(CaptureError::InitFailed(format!(
                "monitor {} does not exist (only {} present)",
                id, count
            )));
        }
        let new_capture = Self::new(0, id)?;
        self.duplication = new_capture.duplication;
        self.device = new_capture.device;
        self.context = new_capture.context;
        self.width = new_capture.width;
        self.height = new_capture.height;
        self.output_index = id;
        // Force staging texture to be recreated for the new monitor.
        self.staging = None;
        Ok(())
    }
}
