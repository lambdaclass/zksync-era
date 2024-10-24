// This is a simplification of Layr-Labs rust-kzg-bn254 library.
// Everything in this file belongs to https://github.com/Layr-Labs/rust-kzg-bn254
// We need to manually copy it because it requires a different rust version.
// We only included necessary parts for the project.

use std::{
    fs::{self, File},
    io::{self, BufReader, Read},
};

use ark_bn254::G1Affine;
use ark_ec::AffineRepr;
use crossbeam_channel::{bounded, Receiver, Sender};

pub trait ReadPointFromBytes: AffineRepr {
    fn read_point_from_bytes_be(bytes: &[u8]) -> io::Result<Self>;
    fn read_point_from_bytes_native_compressed(bytes: &[u8]) -> io::Result<Self>;
}

#[derive(Clone, Debug, PartialEq)]
pub enum KzgError {
    CommitError(String),
    SerializationError(String),
    FftError(String),
    GenericError(String),
}

#[derive(Debug, PartialEq, Clone)]
pub struct Kzg {
    g1: Vec<G1Affine>,
    srs_order: u64,
}

impl Kzg {
    pub fn setup(
        path_to_g1_points: &str,
        srs_order: u32,
        srs_points_to_load: u32,
    ) -> Result<Self, KzgError> {
        if srs_points_to_load > srs_order {
            return Err(KzgError::GenericError(
                "number of points to load is more than the srs order".to_string(),
            ));
        }

        let g1_points =
            Self::parallel_read_g1_points(path_to_g1_points.to_owned(), srs_points_to_load, false)
                .map_err(|e| KzgError::SerializationError(e.to_string()))?;

        Ok(Self {
            g1: g1_points,
            srs_order: srs_order.into(),
        })
    }

    pub fn process_chunks<T>(receiver: Receiver<(Vec<u8>, usize, bool)>) -> Vec<(T, usize)>
    where
        T: ReadPointFromBytes,
    {
        #[allow(clippy::unnecessary_filter_map)]
        receiver
            .iter()
            .map(|(chunk, position, is_native)| {
                let point: T = if is_native {
                    T::read_point_from_bytes_native_compressed(&chunk)
                        .expect("Failed to read point from bytes")
                } else {
                    T::read_point_from_bytes_be(&chunk).expect("Failed to read point from bytes")
                };

                (point, position)
            })
            .collect()
    }

    /// read G1 points in parallel
    pub fn parallel_read_g1_points(
        file_path: String,
        srs_points_to_load: u32,
        is_native: bool,
    ) -> Result<Vec<G1Affine>, KzgError> {
        let (sender, receiver) = bounded::<(Vec<u8>, usize, bool)>(1000);

        // Spawning the reader thread
        let reader_thread = std::thread::spawn(
            move || -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
                Self::read_file_chunks(&file_path, sender, 32, srs_points_to_load, is_native)
                    .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { Box::new(e) })
            },
        );

        let num_workers = num_cpus::get();

        let workers: Vec<_> = (0..num_workers)
            .map(|_| {
                let receiver = receiver.clone();
                std::thread::spawn(move || Self::process_chunks::<G1Affine>(receiver))
            })
            .collect();

        // Wait for the reader thread to finish
        match reader_thread.join() {
            Ok(result) => match result {
                Ok(_) => {}
                Err(e) => return Err(KzgError::GenericError(e.to_string())),
            },
            Err(_) => return Err(KzgError::GenericError("Thread panicked".to_string())),
        }

        // Collect and sort results
        let mut all_points = Vec::new();
        for worker in workers {
            let points = worker.join().expect("Worker thread panicked");
            all_points.extend(points);
        }

        // Sort by original position to maintain order
        all_points.sort_by_key(|&(_, position)| position);

        Ok(all_points.iter().map(|(point, _)| *point).collect())
    }

    /// read files in chunks with specified length
    fn read_file_chunks(
        file_path: &str,
        sender: Sender<(Vec<u8>, usize, bool)>,
        point_size: usize,
        num_points: u32,
        is_native: bool,
    ) -> io::Result<()> {
        let file = File::open(file_path)?;
        let mut reader = BufReader::new(file);
        let mut position = 0;
        let mut buffer = vec![0u8; point_size];

        let mut i = 0;
        while let Ok(bytes_read) = reader.read(&mut buffer) {
            if bytes_read == 0 {
                break;
            }
            sender
                .send((buffer[..bytes_read].to_vec(), position, is_native))
                .unwrap();
            position += bytes_read;
            buffer.resize(point_size, 0); // Ensure the buffer is always the correct size
            i += 1;
            if num_points == i {
                break;
            }
        }
        Ok(())
    }
}
