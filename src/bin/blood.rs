//! Commander Blood — playable, faithful: the ORIGINAL BLOODPRG.EXE running in the path-B
//! runtime, presented in an X11 window with real keyboard + mouse input.
//!
//! Usage:
//!   blood                         # windowed, real-time pacing (~18.2 ticks/s)
//!   blood --turbo                 # windowed, unpaced
//!   blood --script FILE --out DIR # headless: drive with a scripted input timeline
//!
//! Script lines (headless mode): `wait <ticks>`, `key <scan> [ascii]`, `move <x> <y>`,
//! `press <x> <y>`, `release <x> <y>`, `click <x> <y>` (press+release), `shot <name>`.
//! Coordinates are native 320x200; the runtime converts to DOS-virtual mouse coords.

use commander_blood_tools::recomp::runtime::{RunEnd, Runtime, STEPS_PER_SECOND};
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const TICK: Duration = Duration::from_micros(54_925); // 18.2065 Hz PIT

fn make_runtime() -> Runtime {
    let c_root = PathBuf::from("accuracy/cdrive");
    let d_root = PathBuf::from("output/_tmp_iso");
    std::fs::create_dir_all(c_root.join("cblood")).unwrap();
    let exe = std::fs::read(d_root.join("BLOODPRG.EXE")).expect("D:\\BLOODPRG.EXE");
    let mut rt = Runtime::new(c_root, d_root);
    rt.load_exe(&exe, " AMR S162227 EMS WRIC:\\cblood\\", "D:\\BLOODPRG.EXE")
        .unwrap();
    rt
}

/// One BIOS-tick (1/18.2 s) of emulated time, regardless of the guest's PIT rate.
/// Returns false when the guest exited/faulted.
fn run_tick(rt: &mut Runtime) -> Result<bool, String> {
    let target = rt.cpu.steps + STEPS_PER_SECOND * 55 / 1000;
    match rt.run(target) {
        RunEnd::StepBudget => Ok(true),
        RunEnd::Exited(c) => {
            eprintln!("game exited with code {c}");
            Ok(false)
        }
        RunEnd::Fatal(e) => Err(e),
    }
}

fn scancode_ascii(scan: u8) -> u8 {
    // US layout, unshifted — enough for menu/gameplay keys.
    const MAP: &[(u8, u8)] = &[
        (0x01, 27),
        (0x1c, b'\r'),
        (0x39, b' '),
        (0x0e, 8),
        (0x0f, b'\t'),
        (0x02, b'1'),
        (0x03, b'2'),
        (0x04, b'3'),
        (0x05, b'4'),
        (0x06, b'5'),
        (0x07, b'6'),
        (0x08, b'7'),
        (0x09, b'8'),
        (0x0a, b'9'),
        (0x0b, b'0'),
        (0x10, b'q'),
        (0x11, b'w'),
        (0x12, b'e'),
        (0x13, b'r'),
        (0x14, b't'),
        (0x15, b'y'),
        (0x16, b'u'),
        (0x17, b'i'),
        (0x18, b'o'),
        (0x19, b'p'),
        (0x1e, b'a'),
        (0x1f, b's'),
        (0x20, b'd'),
        (0x21, b'f'),
        (0x22, b'g'),
        (0x23, b'h'),
        (0x24, b'j'),
        (0x25, b'k'),
        (0x26, b'l'),
        (0x2c, b'z'),
        (0x2d, b'x'),
        (0x2e, b'c'),
        (0x2f, b'v'),
        (0x30, b'b'),
        (0x31, b'n'),
        (0x32, b'm'),
    ];
    MAP.iter().find(|(s, _)| *s == scan).map(|(_, a)| *a).unwrap_or(0)
}

fn run_script(script_path: &str, out_dir: &PathBuf) -> Result<(), String> {
    std::fs::create_dir_all(out_dir).map_err(|e| e.to_string())?;
    let mut rt = make_runtime();
    let script = std::fs::read_to_string(script_path).map_err(|e| e.to_string())?;
    let (mut mx, mut my, mut mb) = (160u16, 100u16, 0u16);
    for (ln, line) in script.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let w: Vec<&str> = line.split_whitespace().collect();
        let arg = |i: usize| -> u16 { w.get(i).and_then(|s| s.parse().ok()).unwrap_or(0) };
        match w[0] {
            "wait" => {
                for _ in 0..arg(1) {
                    if !run_tick(&mut rt).map_err(|e| format!("line {}: {e}", ln + 1))? {
                        return Ok(());
                    }
                }
            }
            "key" => {
                let scan = arg(1) as u8;
                let ascii = if w.len() > 2 { arg(2) as u8 } else { scancode_ascii(scan) };
                rt.key_event(scan, ascii);
            }
            "move" | "press" | "release" | "click" => {
                mx = arg(1).min(319);
                my = arg(2).min(199);
                match w[0] {
                    "press" => mb = 1,
                    "release" => mb = 0,
                    _ => {}
                }
                rt.mouse_event(mx * 2, my, mb);
                if w[0] == "click" {
                    for _ in 0..2 {
                        if !run_tick(&mut rt)? {
                            return Ok(());
                        }
                    }
                    rt.mouse_event(mx * 2, my, 1);
                    for _ in 0..3 {
                        if !run_tick(&mut rt)? {
                            return Ok(());
                        }
                    }
                    rt.mouse_event(mx * 2, my, 0);
                }
            }
            "shot" => {
                let name = w.get(1).unwrap_or(&"shot");
                rt.write_ppm(&out_dir.join(format!("{name}.ppm"))).map_err(|e| e.to_string())?;
                eprintln!("shot {name} at tick {}", rt.ticks());
            }
            other => return Err(format!("line {}: unknown command {other}", ln + 1)),
        }
    }
    rt.write_ppm(&out_dir.join("end.ppm")).map_err(|e| e.to_string())?;
    if !rt.sb_pcm.is_empty() {
        write_wav(&out_dir.join("end.wav"), &rt.sb_pcm, rt.sb_pcm_rate).map_err(|e| e.to_string())?;
        eprintln!("wav: {} bytes at {} Hz", rt.sb_pcm.len(), rt.sb_pcm_rate);
    }
    Ok(())
}

/// Minimal 8-bit unsigned mono WAV writer for the SB PCM tap.
fn write_wav(path: &std::path::Path, pcm: &[u8], rate: u32) -> std::io::Result<()> {
    let mut d = Vec::with_capacity(44 + pcm.len());
    d.extend_from_slice(b"RIFF");
    d.extend_from_slice(&(36 + pcm.len() as u32).to_le_bytes());
    d.extend_from_slice(b"WAVEfmt ");
    d.extend_from_slice(&16u32.to_le_bytes());
    d.extend_from_slice(&1u16.to_le_bytes()); // PCM
    d.extend_from_slice(&1u16.to_le_bytes()); // mono
    d.extend_from_slice(&rate.to_le_bytes());
    d.extend_from_slice(&rate.to_le_bytes()); // byte rate (8-bit mono)
    d.extend_from_slice(&1u16.to_le_bytes()); // block align
    d.extend_from_slice(&8u16.to_le_bytes()); // bits
    d.extend_from_slice(b"data");
    d.extend_from_slice(&(pcm.len() as u32).to_le_bytes());
    d.extend_from_slice(pcm);
    std::fs::write(path, d)
}

/// Live audio: a ring buffer the emulation fills from the SB PCM tap and a cpal stream
/// drains with naive resampling. Absent an output device the game just runs silent.
struct AudioOut {
    _stream: cpal::Stream,
    ring: Arc<Mutex<VecDeque<u8>>>,
    src_rate: Arc<AtomicU32>,
}

impl AudioOut {
    fn start() -> Option<Self> {
        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
        let host = cpal::default_host();
        let device = host.default_output_device()?;
        let config = device.default_output_config().ok()?;
        let dev_rate = config.sample_rate().0.max(1) as u64;
        let channels = config.channels() as usize;
        let ring: Arc<Mutex<VecDeque<u8>>> = Arc::default();
        let src_rate = Arc::new(AtomicU32::new(11111));
        let (r2, sr) = (Arc::clone(&ring), Arc::clone(&src_rate));
        let mut frac: u64 = 0;
        let mut cur: f32 = 0.0;
        let stream = device
            .build_output_stream(
                &config.config(),
                move |out: &mut [f32], _| {
                    let mut q = r2.lock().unwrap();
                    let step = ((sr.load(Ordering::Relaxed) as u64) << 16) / dev_rate;
                    for frame in out.chunks_mut(channels) {
                        frac += step;
                        while frac >= 1 << 16 {
                            frac -= 1 << 16;
                            match q.pop_front() {
                                Some(b) => cur = (b as f32 - 128.0) / 128.0,
                                None => cur *= 0.995, // underrun: decay to silence, no click
                            }
                        }
                        for s in frame.iter_mut() {
                            *s = cur;
                        }
                    }
                },
                |_| {},
                None,
            )
            .ok()?;
        stream.play().ok()?;
        Some(Self {
            _stream: stream,
            ring,
            src_rate,
        })
    }
}

fn run_window(turbo: bool) -> anyhow::Result<()> {
    use x11rb::connection::Connection;
    use x11rb::protocol::Event;
    use x11rb::protocol::xproto::{
        AtomEnum, ConnectionExt, CreateGCAux, CreateWindowAux, EventMask, ImageFormat, PropMode,
        WindowClass,
    };
    use x11rb::wrapper::ConnectionExt as _;

    let mut rt = make_runtime();
    let (src_w, src_h) = (320usize, 200usize);
    let (conn, screen_num) = x11rb::connect(None)?;
    let screen = &conn.setup().roots[screen_num];
    let (mut win_w, mut win_h) = (960u16, 600u16);
    let win = conn.generate_id()?;
    conn.create_window(
        screen.root_depth,
        win,
        screen.root,
        0,
        0,
        win_w,
        win_h,
        0,
        WindowClass::INPUT_OUTPUT,
        screen.root_visual,
        &CreateWindowAux::new().event_mask(
            EventMask::EXPOSURE
                | EventMask::POINTER_MOTION
                | EventMask::BUTTON_PRESS
                | EventMask::BUTTON_RELEASE
                | EventMask::KEY_PRESS
                | EventMask::STRUCTURE_NOTIFY,
        ),
    )?;
    conn.change_property8(
        PropMode::REPLACE,
        win,
        u32::from(AtomEnum::WM_NAME),
        u32::from(AtomEnum::STRING),
        b"Commander Blood",
    )?;
    conn.map_window(win)?;
    let gc = conn.generate_id()?;
    conn.create_gc(gc, win, &CreateGCAux::new())?;
    conn.flush()?;

    let audio = AudioOut::start();
    let mut pcm_consumed = 0usize;
    let max_req = 262_144usize;
    let (mut mx, mut my, mut mb) = (160u16, 100u16, 0u16);
    let mut next_frame = Instant::now();
    loop {
        let scale = ((win_w as usize / src_w).min(win_h as usize / src_h)).max(1);
        let (dst_w, dst_h) = (src_w * scale, src_h * scale);
        let off_x = (win_w as usize).saturating_sub(dst_w) / 2;
        let off_y = (win_h as usize).saturating_sub(dst_h) / 2;
        let to_src = |ex: i16, ey: i16| -> (u16, u16) {
            let x = ((ex as isize - off_x as isize).clamp(0, dst_w as isize - 1) as usize / scale)
                .min(src_w - 1) as u16;
            let y = ((ey as isize - off_y as isize).clamp(0, dst_h as isize - 1) as usize / scale)
                .min(src_h - 1) as u16;
            (x, y)
        };
        while let Some(event) = conn.poll_for_event()? {
            match event {
                Event::MotionNotify(m) => {
                    (mx, my) = to_src(m.event_x, m.event_y);
                    rt.mouse_event(mx * 2, my, mb);
                }
                Event::ButtonPress(b) if b.detail == 1 || b.detail == 3 => {
                    mb |= if b.detail == 1 { 1 } else { 2 };
                    rt.mouse_event(mx * 2, my, mb);
                }
                Event::ButtonRelease(b) if b.detail == 1 || b.detail == 3 => {
                    mb &= if b.detail == 1 { !1 } else { !2 };
                    rt.mouse_event(mx * 2, my, mb);
                }
                Event::KeyPress(k) => {
                    // X evdev keycodes are PC scancodes + 8 for the classic set.
                    let scan = k.detail.saturating_sub(8);
                    rt.key_event(scan, scancode_ascii(scan));
                }
                Event::ConfigureNotify(c) => {
                    if c.width != win_w || c.height != win_h {
                        win_w = c.width;
                        win_h = c.height;
                    }
                }
                _ => {}
            }
        }

        match run_tick(&mut rt) {
            Ok(true) => {}
            Ok(false) => return Ok(()),
            Err(e) => anyhow::bail!("runtime fault: {e}"),
        }

        // Feed freshly streamed SB PCM to the audio ring (cap ~2 s to bound turbo runaway).
        if let Some(a) = &audio {
            if rt.sb_pcm.len() > pcm_consumed {
                let mut q = a.ring.lock().unwrap();
                q.extend(&rt.sb_pcm[pcm_consumed..]);
                let cap = (rt.sb_pcm_rate as usize) * 2;
                while q.len() > cap {
                    q.pop_front();
                }
                a.src_rate.store(rt.sb_pcm_rate, Ordering::Relaxed);
            }
        }
        pcm_consumed = rt.sb_pcm.len();

        // Present: scale the 320x200 frame into a BGRX image, letterboxed.
        let rgb = rt.screenshot_rgb();
        let mut image = vec![0u8; win_w as usize * win_h as usize * 4];
        for y in 0..dst_h.min(win_h as usize) {
            let sy = y / scale;
            for x in 0..dst_w.min(win_w as usize) {
                let sx = x / scale;
                let si = (sy * src_w + sx) * 3;
                let di = ((y + off_y) * win_w as usize + x + off_x) * 4;
                image[di] = rgb[si + 2];
                image[di + 1] = rgb[si + 1];
                image[di + 2] = rgb[si];
            }
        }
        let stride = win_w as usize * 4;
        let rows_per_chunk = (max_req / stride).max(1);
        let mut row = 0usize;
        while row < win_h as usize {
            let n = rows_per_chunk.min(win_h as usize - row);
            conn.put_image(
                ImageFormat::Z_PIXMAP,
                win,
                gc,
                win_w,
                n as u16,
                0,
                row as i16,
                0,
                screen.root_depth,
                &image[row * stride..(row + n) * stride],
            )?;
            row += n;
        }
        conn.flush()?;

        if !turbo {
            next_frame += TICK;
            let now = Instant::now();
            if next_frame > now {
                std::thread::sleep(next_frame - now);
            } else {
                next_frame = now;
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut script: Option<String> = None;
    let mut out = PathBuf::from("blood_shots");
    let mut turbo = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--script" => {
                i += 1;
                script = Some(args[i].clone());
            }
            "--out" => {
                i += 1;
                out = PathBuf::from(&args[i]);
            }
            "--turbo" => turbo = true,
            a => anyhow::bail!("unknown arg {a}"),
        }
        i += 1;
    }
    match script {
        Some(s) => run_script(&s, &out).map_err(|e| anyhow::anyhow!(e)),
        None => run_window(turbo),
    }
}
