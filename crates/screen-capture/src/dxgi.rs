use crate::{error::CaptureError, Frame, PixelFormat, ScreenCapture};
use std::time::{SystemTime, UNIX_EPOCH};
use windows::core::{ComInterface};
use windows::Win32::Graphics::Direct3D::{D3D_DRIVER_TYPE_HARDWARE, D3D_FEATURE_LEVEL_11_0};
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, D3D11_CREATE_DEVICE_FLAG, D3D11_SDK_VERSION,
};
use windows::Win32::Graphics::Dxgi::{
    IDXGIAdapter, IDXGIDevice, IDXGIOutput, IDXGIOutput1, IDXGIOutputDuplication, DXGI_OUTDUPL_FRAME_INFO,
};

pub struct WindowsScreenCapture {
    duplication: IDXGIOutputDuplication,
    _device: ID3D11Device,
    _context: ID3D11DeviceContext,
    width: u32,
    height: u32,
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
            ).map_err(|e| CaptureError::InitFailed(e.to_string()))?;

            let device = device.unwrap();
            let context = context.unwrap();

            // 2. Get DXGI Device, Adapter, Factory
            let dxgi_device: IDXGIDevice = device.cast().map_err(|e| CaptureError::InitFailed(e.to_string()))?;
            let dxgi_adapter: IDXGIAdapter = dxgi_device.GetAdapter().map_err(|e| CaptureError::InitFailed(e.to_string()))?;
            
            // 3. Get Specific Output (Monitor)
            let dxgi_output: IDXGIOutput = dxgi_adapter.EnumOutputs(output_index).map_err(|e| CaptureError::InitFailed(e.to_string()))?;
            let dxgi_output1: IDXGIOutput1 = dxgi_output.cast().map_err(|e| CaptureError::InitFailed(e.to_string()))?;

            // 4. Duplicate Output
            let duplication = dxgi_output1.DuplicateOutput(&device).map_err(|e| CaptureError::InitFailed(e.to_string()))?;

            let mut desc = Default::default();
            duplication.GetDesc(&mut desc);

            Ok(Self {
                duplication,
                _device: device,
                _context: context,
                width: 1920, // In real code, parse from desc
                height: 1080,
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
                None, D3D_DRIVER_TYPE_HARDWARE, None, D3D11_CREATE_DEVICE_FLAG(0),
                Some(&feature_levels), D3D11_SDK_VERSION, Some(&mut device), None, Some(&mut context),
            ).is_ok() {
                if let Some(device) = device {
                    if let Ok(dxgi_device) = device.cast::<IDXGIDevice>() {
                        if let Ok(dxgi_adapter) = dxgi_device.GetAdapter() {
                            let mut i = 0;
                            while let Ok(output) = dxgi_adapter.EnumOutputs(i) {
                                let mut desc = windows::Win32::Graphics::Dxgi::DXGI_OUTPUT_DESC::default();
                                if output.GetDesc(&mut desc).is_ok() {
                                    let name_len = desc.DeviceName.iter().position(|&c| c == 0).unwrap_or(desc.DeviceName.len());
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
}

impl ScreenCapture for WindowsScreenCapture {
    fn capture_frame(&mut self) -> Result<Frame, CaptureError> {
        unsafe {
            let mut frame_info = DXGI_OUTDUPL_FRAME_INFO::default();
            let mut resource = None;
            
            match self.duplication.AcquireNextFrame(250, &mut frame_info, &mut resource) {
                Ok(_) => {
                    let _ = self.duplication.ReleaseFrame();
                    
                    Ok(Frame {
                        data: vec![0; (self.width * self.height * 4) as usize], // Fake frame copy
                        width: self.width,
                        height: self.height,
                        pixel_format: PixelFormat::Bgra8,
                        timestamp_ms: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64,
                        monitor_id: 0,
                    })
                }
                Err(e) => {
                    if e.code().0 == 0x887A0027u32 as i32 { // DXGI_ERROR_WAIT_TIMEOUT
                        Err(CaptureError::Timeout)
                    } else if e.code().0 == 0x887A0026u32 as i32 { // DXGI_ERROR_ACCESS_LOST
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
        let new_capture = Self::new(0, id)?;
        self.duplication = new_capture.duplication;
        self.width = new_capture.width;
        self.height = new_capture.height;
        Ok(())
    }
}
