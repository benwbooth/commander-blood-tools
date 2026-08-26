//! Startup cleanup of optional transient files.

/// Number of transient path slots in the original startup table.
pub const STARTUP_TRANSIENT_PATH_COUNT: usize = 4;
const PRESERVE_PATH_MARKER: u8 = b'x';

/// Host filesystem operation used by startup cleanup.
pub trait StartupTransientFileHost {
    /// Request deletion of one authored path. Native deletion errors are ignored.
    fn remove_transient_file(&mut self, path: &str);
}

/// Translate BLOODPRG routine `0x00147F` over owned host paths.
///
/// Exactly four slots are visited in order. A path is preserved only when its
/// first byte is lowercase `x`; uppercase markers and empty paths still issue
/// the original deletion request.
pub fn delete_startup_transient_files<H: StartupTransientFileHost>(
    paths: &[String; STARTUP_TRANSIENT_PATH_COUNT],
    host: &mut H,
) {
    for path in paths {
        if path.as_bytes().first().copied() != Some(PRESERVE_PATH_MARKER) {
            host.remove_transient_file(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    struct CleanupVector {
        name: String,
        paths: [String; STARTUP_TRANSIENT_PATH_COUNT],
        delete_calls: Vec<DeleteCall>,
    }

    #[derive(Deserialize)]
    struct DeleteCall {
        path: String,
    }

    #[derive(Default)]
    struct RecordingHost {
        paths: Vec<String>,
    }

    impl StartupTransientFileHost for RecordingHost {
        fn remove_transient_file(&mut self, path: &str) {
            self.paths.push(path.to_owned());
        }
    }

    #[test]
    fn cleanup_matches_every_original_path_vector() {
        let vectors: Vec<CleanupVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_147f_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), 4);

        for vector in vectors {
            let mut host = RecordingHost::default();
            delete_startup_transient_files(&vector.paths, &mut host);
            assert_eq!(
                host.paths,
                vector
                    .delete_calls
                    .into_iter()
                    .map(|call| call.path)
                    .collect::<Vec<_>>(),
                "{}",
                vector.name
            );
        }
    }
}
