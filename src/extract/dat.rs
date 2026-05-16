use super::*;

// ===========================================================================
// blood.dat parser
// ===========================================================================

pub(super) fn extract_dat(dat: &Path, out_dir: &Path) -> Result<u32, Box<dyn Error>> {
    let mut f = File::open(dat)?;
    let mut count = 0u32;

    f.seek(SeekFrom::Start(2))?;

    loop {
        if f.stream_position()? >= 65536 {
            break;
        }

        let mut name_buf = [0u8; 16];
        if f.read_exact(&mut name_buf).is_err() {
            break;
        }
        let name_len = name_buf.iter().position(|&b| b == 0).unwrap_or(16);
        if name_len == 0 {
            break;
        }
        let name = String::from_utf8_lossy(&name_buf[..name_len])
            .to_lowercase()
            .replace('\\', "/");

        let mut buf4 = [0u8; 4];
        f.read_exact(&mut buf4)?;
        let size = i32::from_le_bytes(buf4);
        f.read_exact(&mut buf4)?;
        let offset = i32::from_le_bytes(buf4);

        f.seek(SeekFrom::Current(1))?;

        if size <= 0 || offset < 0 {
            continue;
        }

        let resume = f.stream_position()?;

        let out_path = out_dir.join(&name);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent).ok();
        }

        f.seek(SeekFrom::Start(offset as u64))?;
        let mut data = vec![0u8; size as usize];
        if f.read_exact(&mut data).is_ok() {
            if let Ok(mut out) = File::create(&out_path) {
                let _ = out.write_all(&data);
                count += 1;
            }
        }

        f.seek(SeekFrom::Start(resume))?;
    }

    Ok(count)
}
