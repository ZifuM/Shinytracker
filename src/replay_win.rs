//! Shiny Replay's Windows parts (with the `replay-rec` feature):
//! - Capture: Windows Graphics Capture of the game window only (the same
//!   thing OBS's "Window Capture" uses), cropped to the game's picture, read
//!   back as BGRA. It only looks at the window: the game's files and memory
//!   are never touched.
//! - Encoder: Media Foundation's H.264 encoder (built into Windows; it uses
//!   the graphics card's encoder when there is one) writing one short MP4
//!   segment at a time into memory.
//! - Mux: joins segments into one MP4 file, copying the video as it is (no
//!   second encode).

use crate::replay::{Capture, Encoder, Frame};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;
use std::time::Duration;

use windows::core::{Interface, HSTRING};
use windows::Graphics::Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession};
use windows::Graphics::DirectX::Direct3D11::IDirect3DDevice;
use windows::Graphics::DirectX::DirectXPixelFormat;
use windows::Graphics::SizeInt32;
use windows::Win32::Foundation::{HMODULE, HWND, POINT, RECT};
use windows::Win32::Graphics::Direct3D::D3D_DRIVER_TYPE_HARDWARE;
use windows::Win32::Graphics::Direct3D11::{
    D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, ID3D11Texture2D, D3D11_BOX, D3D11_CPU_ACCESS_READ, D3D11_CREATE_DEVICE_BGRA_SUPPORT, D3D11_MAPPED_SUBRESOURCE, D3D11_MAP_READ,
    D3D11_SDK_VERSION, D3D11_TEXTURE2D_DESC, D3D11_USAGE_STAGING,
};
use windows::Win32::Graphics::Dwm::{DwmGetWindowAttribute, DWMWA_EXTENDED_FRAME_BOUNDS};
use windows::Win32::Graphics::Dxgi::Common::{DXGI_FORMAT_B8G8R8A8_UNORM, DXGI_SAMPLE_DESC};
use windows::Win32::Graphics::Dxgi::IDXGIDevice;
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::Win32::Media::MediaFoundation::*;
use windows::Win32::System::Com::StructuredStorage::CreateStreamOnHGlobal;
use windows::Win32::System::Com::{CoInitializeEx, IStream, COINIT_MULTITHREADED, STATFLAG_NONAME, STATSTG, STREAM_SEEK_SET};
use windows::Win32::System::Threading::{GetCurrentThread, SetThreadPriority, THREAD_PRIORITY_BELOW_NORMAL};
use windows::Win32::System::WinRT::Direct3D11::{CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess};
use windows::Win32::System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop;
use windows::Win32::UI::WindowsAndMessaging::{GetClientRect, IsIconic, IsWindow};

fn e(what: &str, err: windows::core::Error) -> String {
    format!("{what}: {}", err.message())
}

/// COM on this thread (each recorder thread calls it once; harmless twice).
fn com() {
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
    }
}

/// Media Foundation, once for the whole app.
fn mf() -> Result<(), String> {
    static START: Once = Once::new();
    static OK: AtomicBool = AtomicBool::new(false);
    START.call_once(|| {
        OK.store(unsafe { MFStartup(MF_VERSION, MFSTARTUP_FULL) }.is_ok(), Ordering::SeqCst);
    });
    if OK.load(Ordering::SeqCst) {
        Ok(())
    } else {
        Err("Windows' video tools (Media Foundation) aren't available on this PC. On Windows N editions, install the Media Feature Pack.".into())
    }
}

/// The recorder threads run below normal priority, so counting comes first.
pub fn background() {
    unsafe {
        let _ = SetThreadPriority(GetCurrentThread(), THREAD_PRIORITY_BELOW_NORMAL);
    }
}

// ───────────────────────────── capture ─────────────────────────────

pub fn capture() -> Result<Box<dyn Capture>, String> {
    com();
    if !GraphicsCaptureSession::IsSupported().unwrap_or(false) {
        return Err("Window capture needs Windows 10 (version 1903) or newer.".into());
    }
    Ok(Box::new(Wgc::default()))
}

#[derive(Default)]
struct Wgc {
    hwnd: Option<HWND>,
    device: Option<ID3D11Device>,
    ctx: Option<ID3D11DeviceContext>,
    d3d: Option<IDirect3DDevice>,
    pool: Option<Direct3D11CaptureFramePool>,
    session: Option<GraphicsCaptureSession>,
    size: SizeInt32,
    staging: Option<(ID3D11Texture2D, u32, u32)>,
}

impl Capture for Wgc {
    fn open(&mut self) -> Result<(), String> {
        let h = crate::topmost::game_hwnd().ok_or("Open the game: the recorder starts when its window is there.")?;
        let hwnd = HWND(h as *mut core::ffi::c_void);
        unsafe {
            if IsIconic(hwnd).as_bool() {
                return Err("The game is minimised.".into());
            }
            let mut device = None;
            let mut ctx = None;
            D3D11CreateDevice(None, D3D_DRIVER_TYPE_HARDWARE, HMODULE::default(), D3D11_CREATE_DEVICE_BGRA_SUPPORT, None, D3D11_SDK_VERSION, Some(&mut device), None, Some(&mut ctx))
                .map_err(|x| e("Graphics device", x))?;
            let device: ID3D11Device = device.ok_or("No graphics device")?;
            let ctx = ctx.ok_or("No graphics device context")?;
            let dxgi: IDXGIDevice = device.cast().map_err(|x| e("Graphics device", x))?;
            let d3d: IDirect3DDevice = CreateDirect3D11DeviceFromDXGIDevice(&dxgi).and_then(|i| i.cast()).map_err(|x| e("Graphics device", x))?;
            let interop = windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>().map_err(|x| e("Window capture", x))?;
            let item: GraphicsCaptureItem = interop.CreateForWindow(hwnd).map_err(|x| e("Capturing the game window", x))?;
            let size = item.Size().map_err(|x| e("Window size", x))?;
            let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(&d3d, DirectXPixelFormat::B8G8R8A8UIntNormalized, 2, size).map_err(|x| e("Window capture", x))?;
            let session = pool.CreateCaptureSession(&item).map_err(|x| e("Window capture", x))?;
            // no mouse pointer, and no yellow border where Windows allows it
            let _ = session.SetIsCursorCaptureEnabled(false);
            let _ = session.SetIsBorderRequired(false);
            session.StartCapture().map_err(|x| e("Starting window capture", x))?;
            *self = Wgc { hwnd: Some(hwnd), device: Some(device), ctx: Some(ctx), d3d: Some(d3d), pool: Some(pool), session: Some(session), size, staging: None };
        }
        Ok(())
    }

    fn grab(&mut self) -> Result<Option<Frame>, String> {
        let (Some(hwnd), Some(pool)) = (self.hwnd, self.pool.clone()) else { return Err("Not started".into()) };
        unsafe {
            if !IsWindow(hwnd).as_bool() {
                return Err("The game was closed.".into());
            }
        }
        // the newest picture (older ones are dropped)
        let mut frame = None;
        while let Ok(f) = pool.TryGetNextFrame() {
            frame = Some(f);
        }
        let Some(frame) = frame else { return Ok(None) };
        let content = frame.ContentSize().map_err(|x| e("Window capture", x))?;
        if content.Width != self.size.Width || content.Height != self.size.Height {
            // the window changed size: new buffers for the next pictures
            if let Some(d3d) = &self.d3d {
                let _ = pool.Recreate(d3d, DirectXPixelFormat::B8G8R8A8UIntNormalized, 2, content);
            }
            self.size = content;
        }
        let tex: ID3D11Texture2D = unsafe {
            frame.Surface().and_then(|s| s.cast::<IDirect3DDxgiInterfaceAccess>()).and_then(|a| a.GetInterface()).map_err(|x| e("Window capture", x))?
        };
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        unsafe { tex.GetDesc(&mut desc) };
        let (tw, th) = (desc.Width.min(content.Width.max(1) as u32), desc.Height.min(content.Height.max(1) as u32));
        // just the game's picture: no title bar or borders
        let (x0, y0, w, h) = client_box(hwnd).map(|(x, y, w, h)| (x.min(tw), y.min(th), w.min(tw - x.min(tw)), h.min(th - y.min(th)))).filter(|b| b.2 >= 16 && b.3 >= 16).unwrap_or((0, 0, tw, th));
        if w < 2 || h < 2 {
            return Ok(None);
        }
        let (Some(device), Some(ctx)) = (&self.device, &self.ctx) else { return Err("Not started".into()) };
        if !self.staging.as_ref().is_some_and(|(_, sw, sh)| *sw == w && *sh == h) {
            let sd = D3D11_TEXTURE2D_DESC {
                Width: w,
                Height: h,
                MipLevels: 1,
                ArraySize: 1,
                Format: DXGI_FORMAT_B8G8R8A8_UNORM,
                SampleDesc: DXGI_SAMPLE_DESC { Count: 1, Quality: 0 },
                Usage: D3D11_USAGE_STAGING,
                BindFlags: 0,
                CPUAccessFlags: D3D11_CPU_ACCESS_READ.0 as u32,
                MiscFlags: 0,
            };
            let mut st = None;
            unsafe { device.CreateTexture2D(&sd, None, Some(&mut st)) }.map_err(|x| e("Graphics memory", x))?;
            self.staging = st.map(|t| (t, w, h));
        }
        let Some((staging, _, _)) = &self.staging else { return Err("Graphics memory".into()) };
        let bx = D3D11_BOX { left: x0, top: y0, front: 0, right: x0 + w, bottom: y0 + h, back: 1 };
        let mut px = vec![0u8; (w * h * 4) as usize];
        unsafe {
            ctx.CopySubresourceRegion(staging, 0, 0, 0, 0, &tex, 0, Some(&bx));
            let mut m = D3D11_MAPPED_SUBRESOURCE::default();
            ctx.Map(staging, 0, D3D11_MAP_READ, 0, Some(&mut m)).map_err(|x| e("Reading the picture", x))?;
            let row = (w * 4) as usize;
            for y in 0..h as usize {
                let src = std::slice::from_raw_parts((m.pData as *const u8).add(y * m.RowPitch as usize), row);
                px[y * row..(y + 1) * row].copy_from_slice(src);
            }
            ctx.Unmap(staging, 0);
        }
        // (WGC leaves alpha undefined for some windows)
        for a in px.iter_mut().skip(3).step_by(4) {
            *a = 255;
        }
        Ok(Some(Frame { w, h, px }))
    }

    fn close(&mut self) {
        if let Some(s) = self.session.take() {
            let _ = s.Close();
        }
        if let Some(p) = self.pool.take() {
            let _ = p.Close();
        }
        *self = Wgc::default();
    }
}

impl Drop for Wgc {
    fn drop(&mut self) {
        if self.session.is_some() || self.pool.is_some() {
            self.close();
        }
    }
}

/// Where the game's picture (its client area) sits inside the captured
/// window image: (x, y, width, height) in pixels.
fn client_box(hwnd: HWND) -> Option<(u32, u32, u32, u32)> {
    unsafe {
        let mut frame = RECT::default();
        DwmGetWindowAttribute(hwnd, DWMWA_EXTENDED_FRAME_BOUNDS, &mut frame as *mut RECT as *mut core::ffi::c_void, std::mem::size_of::<RECT>() as u32).ok()?;
        let mut cr = RECT::default();
        GetClientRect(hwnd, &mut cr).ok()?;
        let mut tl = POINT { x: 0, y: 0 };
        if !ClientToScreen(hwnd, &mut tl).as_bool() {
            return None;
        }
        let (x, y) = (tl.x - frame.left, tl.y - frame.top);
        if x < 0 || y < 0 || cr.right <= 0 || cr.bottom <= 0 {
            return None;
        }
        Some((x as u32, y as u32, cr.right as u32, cr.bottom as u32))
    }
}

// ───────────────────────────── encoder ─────────────────────────────

pub fn encoder() -> Result<Box<dyn Encoder>, String> {
    com();
    mf()?;
    Ok(Box::new(MfEncoder::default()))
}

#[derive(Default)]
struct MfEncoder {
    writer: Option<IMFSinkWriter>,
    stream: Option<IStream>,
    index: u32,
    size: (u32, u32),
    frame_100ns: i64,
}

fn attrs(n: u32) -> Result<IMFAttributes, String> {
    let mut a = None;
    unsafe { MFCreateAttributes(&mut a, n) }.map_err(|x| e("Video settings", x))?;
    a.ok_or_else(|| "Video settings".to_string())
}

fn pack(hi: u32, lo: u32) -> u64 {
    ((hi as u64) << 32) | lo as u64
}

/// A video media type: H.264 (`kbps` > 0) or RGB32 pictures.
fn video_type(w: u32, h: u32, fps: u32, kbps: u32) -> windows::core::Result<IMFMediaType> {
    unsafe {
        let t = MFCreateMediaType()?;
        t.SetGUID(&MF_MT_MAJOR_TYPE, &MFMediaType_Video)?;
        t.SetUINT32(&MF_MT_INTERLACE_MODE, MFVideoInterlace_Progressive.0 as u32)?;
        t.SetUINT64(&MF_MT_FRAME_SIZE, pack(w, h))?;
        t.SetUINT64(&MF_MT_FRAME_RATE, pack(fps, 1))?;
        t.SetUINT64(&MF_MT_PIXEL_ASPECT_RATIO, pack(1, 1))?;
        if kbps > 0 {
            t.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_H264)?;
            t.SetUINT32(&MF_MT_AVG_BITRATE, kbps.saturating_mul(1000))?;
            t.SetUINT32(&MF_MT_MPEG2_PROFILE, eAVEncH264VProfile_High.0 as u32)?;
        } else {
            t.SetGUID(&MF_MT_SUBTYPE, &MFVideoFormat_RGB32)?;
            // rows top to bottom
            t.SetUINT32(&MF_MT_DEFAULT_STRIDE, w * 4)?;
        }
        Ok(t)
    }
}

/// A growable stream in memory.
fn mem_stream() -> Result<IStream, String> {
    unsafe { CreateStreamOnHGlobal(windows::Win32::Foundation::HGLOBAL::default(), true) }.map_err(|x| e("Memory for the video", x))
}

fn stream_bytes(s: &IStream) -> Result<Vec<u8>, String> {
    unsafe {
        let mut st = STATSTG::default();
        s.Stat(&mut st, STATFLAG_NONAME).map_err(|x| e("Reading the video", x))?;
        let n = st.cbSize as usize;
        s.Seek(0, STREAM_SEEK_SET, None).map_err(|x| e("Reading the video", x))?;
        let mut out = vec![0u8; n];
        let mut got = 0usize;
        while got < n {
            let mut read = 0u32;
            let hr = s.Read(out[got..].as_mut_ptr() as *mut core::ffi::c_void, (n - got).min(1 << 24) as u32, Some(&mut read));
            if hr.is_err() || read == 0 {
                break;
            }
            got += read as usize;
        }
        out.truncate(got);
        Ok(out)
    }
}

impl Encoder for MfEncoder {
    fn begin(&mut self, w: u32, h: u32, fps: u32, kbps: u32) -> Result<(), String> {
        let stream = mem_stream()?;
        unsafe {
            let bs = MFCreateMFByteStreamOnStream(&stream).map_err(|x| e("Memory for the video", x))?;
            let a = attrs(4)?;
            a.SetUINT32(&MF_READWRITE_ENABLE_HARDWARE_TRANSFORMS, 1).map_err(|x| e("Video settings", x))?;
            a.SetGUID(&MF_TRANSCODE_CONTAINERTYPE, &MFTranscodeContainerType_MPEG4).map_err(|x| e("Video settings", x))?;
            a.SetUINT32(&MF_SINK_WRITER_DISABLE_THROTTLING, 1).map_err(|x| e("Video settings", x))?;
            let writer = MFCreateSinkWriterFromURL(&HSTRING::from("replay.mp4"), &bs, &a).map_err(|x| e("Starting the video encoder", x))?;
            let out = video_type(w, h, fps, kbps).map_err(|x| e("Video format", x))?;
            let input = video_type(w, h, fps, 0).map_err(|x| e("Video format", x))?;
            let index = writer.AddStream(&out).map_err(|x| e("Video encoder (H.264)", x))?;
            writer.SetInputMediaType(index, &input, None).map_err(|x| e("Video encoder input", x))?;
            writer.BeginWriting().map_err(|x| e("Starting the video encoder", x))?;
            self.writer = Some(writer);
            self.stream = Some(stream);
            self.index = index;
        }
        self.size = (w, h);
        self.frame_100ns = 10_000_000 / fps.max(1) as i64;
        Ok(())
    }

    fn frame(&mut self, bgra: &[u8], t: Duration) -> Result<(), String> {
        let Some(writer) = &self.writer else { return Err("The encoder isn't started".into()) };
        let len = (self.size.0 * self.size.1 * 4) as usize;
        if bgra.len() < len {
            return Ok(());
        }
        unsafe {
            let buf = MFCreateMemoryBuffer(len as u32).map_err(|x| e("Video memory", x))?;
            let mut p: *mut u8 = std::ptr::null_mut();
            buf.Lock(&mut p, None, None).map_err(|x| e("Video memory", x))?;
            std::ptr::copy_nonoverlapping(bgra.as_ptr(), p, len);
            buf.Unlock().map_err(|x| e("Video memory", x))?;
            buf.SetCurrentLength(len as u32).map_err(|x| e("Video memory", x))?;
            let s = MFCreateSample().map_err(|x| e("Video memory", x))?;
            s.AddBuffer(&buf).map_err(|x| e("Video memory", x))?;
            s.SetSampleTime((t.as_nanos() / 100) as i64).map_err(|x| e("Video timing", x))?;
            s.SetSampleDuration(self.frame_100ns).map_err(|x| e("Video timing", x))?;
            writer.WriteSample(self.index, &s).map_err(|x| e("Encoding a frame", x))?;
        }
        Ok(())
    }

    fn finish(&mut self) -> Result<Vec<u8>, String> {
        let writer = self.writer.take().ok_or("The encoder isn't started")?;
        unsafe { writer.Finalize() }.map_err(|x| e("Finishing the video", x))?;
        drop(writer);
        let stream = self.stream.take().ok_or("No video")?;
        stream_bytes(&stream)
    }
}

// ───────────────────────────── joining ─────────────────────────────

/// A source reader on MP4 bytes in memory.
fn reader_on(bytes: &[u8]) -> Result<IMFSourceReader, String> {
    let stream = mem_stream()?;
    unsafe {
        let mut done = 0u32;
        stream.Write(bytes.as_ptr() as *const core::ffi::c_void, bytes.len() as u32, Some(&mut done)).ok().map_err(|x| e("Reading a segment", x))?;
        stream.Seek(0, STREAM_SEEK_SET, None).map_err(|x| e("Reading a segment", x))?;
        let bs = MFCreateMFByteStreamOnStream(&stream).map_err(|x| e("Reading a segment", x))?;
        let a = attrs(1)?;
        MFCreateSourceReaderFromByteStream(&bs, &a).map_err(|x| e("Reading a segment", x))
    }
}

const VIDEO: u32 = MF_SOURCE_READER_FIRST_VIDEO_STREAM.0 as u32;

/// Join the segments into one MP4 at `path` (the video is copied as is).
pub fn mux(parts: &[Vec<u8>], path: &Path) -> Result<(), String> {
    com();
    mf()?;
    let name = HSTRING::from(path.as_os_str());
    unsafe {
        let file = MFCreateFile(MF_ACCESSMODE_WRITE, MF_OPENMODE_DELETE_IF_EXIST, MF_FILEFLAGS_NONE, &name).map_err(|x| e("Creating the clip file", x))?;
        let a = attrs(1)?;
        a.SetGUID(&MF_TRANSCODE_CONTAINERTYPE, &MFTranscodeContainerType_MPEG4).map_err(|x| e("Video settings", x))?;
        let writer = MFCreateSinkWriterFromURL(&name, &file, &a).map_err(|x| e("Writing the clip", x))?;
        let mut index = None;
        let mut offset: i64 = 0;
        for part in parts {
            let reader = reader_on(part)?;
            if index.is_none() {
                let t = reader.GetNativeMediaType(VIDEO, 0).map_err(|x| e("Reading a segment", x))?;
                let i = writer.AddStream(&t).map_err(|x| e("Writing the clip", x))?;
                writer.SetInputMediaType(i, &t, None).map_err(|x| e("Writing the clip", x))?;
                writer.BeginWriting().map_err(|x| e("Writing the clip", x))?;
                index = Some(i);
            }
            let i = index.expect("set above");
            let mut end = offset;
            loop {
                let mut flags = 0u32;
                let mut ts = 0i64;
                let mut sample: Option<IMFSample> = None;
                reader.ReadSample(VIDEO, 0, None, Some(&mut flags), Some(&mut ts), Some(&mut sample)).map_err(|x| e("Reading a segment", x))?;
                if let Some(s) = sample {
                    let t = s.GetSampleTime().unwrap_or(ts);
                    let d = s.GetSampleDuration().unwrap_or(0);
                    s.SetSampleTime(t + offset).map_err(|x| e("Video timing", x))?;
                    writer.WriteSample(i, &s).map_err(|x| e("Writing the clip", x))?;
                    end = end.max(t + offset + d);
                }
                if flags & MF_SOURCE_READERF_ENDOFSTREAM.0 as u32 != 0 {
                    break;
                }
            }
            offset = end;
        }
        if index.is_none() {
            return Err("Nothing to save".into());
        }
        writer.Finalize().map_err(|x| e("Finishing the clip", x))?;
    }
    Ok(())
}
