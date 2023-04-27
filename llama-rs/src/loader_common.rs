use std::{
    fmt::{Display, Formatter},
    path::{Path, PathBuf},
};

use ggml_format::ContainerType;
use thiserror::Error;

use crate::{util::FindAllModelFilesError, Hyperparameters};

/// How the tensors are stored in the GGML LLaMA model.
#[derive(Debug, PartialEq, Clone, Copy, Eq, Default)]
pub enum FileType {
    /// All tensors are stored as f32.
    F32,
    #[default]
    /// All tensors are mostly stored as `f16`, except for the 1D tensors (32-bit).
    MostlyF16,
    /// All tensors are mostly stored as `Q4_0`, except for the 1D tensors (32-bit).
    MostlyQ4_0,
    /// All tensors are mostly stored as `Q4_1`, except for the 1D tensors (32-bit)
    MostlyQ4_1,
    /// All tensors are mostly stored as `Q4_2`, except for the 1D tensors (32-bit).
    MostlyQ4_2,
    /// All tensors are mostly stored as `Q4_3`, except for the 1D tensors (32-bit).
    MostlyQ4_3,

    MostlyQ5_0,
    MostlyQ5_1,
    MostlyQ8_0,
    MostlyQ8_1,    
}
impl From<FileType> for i32 {
    fn from(value: FileType) -> Self {
        match value {
            FileType::F32 => 0,
            FileType::MostlyF16 => 1,
            FileType::MostlyQ4_0 => 2,
            FileType::MostlyQ4_1 => 3,
            FileType::MostlyQ4_2 => 4,
            FileType::MostlyQ4_3 => 5,
            FileType::MostlyQ5_0 => 6,
            FileType::MostlyQ5_1 => 7,
            FileType::MostlyQ8_0 => 8,
            FileType::MostlyQ8_1 => 9, 
        }
    }
}
impl TryFrom<i32> for FileType {
    type Error = ();

    fn try_from(value: i32) -> Result<Self, Self::Error> {
        match value {
            0 => Ok(FileType::F32),
            1 => Ok(FileType::MostlyF16),
            2 => Ok(FileType::MostlyQ4_0),
            3 => Ok(FileType::MostlyQ4_1),
            4 => Ok(FileType::MostlyQ4_2),
            5 => Ok(FileType::MostlyQ4_3),
            6 => Ok(FileType::MostlyQ5_0),
            7 => Ok(FileType::MostlyQ5_1),
            8 => Ok(FileType::MostlyQ8_0),
            9 => Ok(FileType::MostlyQ8_1),
            _ => Err(()),
        }
    }
}
impl Display for FileType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FileType::F32 => write!(f, "f32"),
            FileType::MostlyF16 => write!(f, "f16"),
            FileType::MostlyQ4_0 => write!(f, "q4_0"),
            FileType::MostlyQ4_1 => write!(f, "q4_1"),
            FileType::MostlyQ4_2 => write!(f, "q4_2"),
            FileType::MostlyQ4_3 => write!(f, "q4_3"),
            FileType::MostlyQ5_0 => write!(f, "q5_0"),
            FileType::MostlyQ5_1 => write!(f, "q5_1"),
            FileType::MostlyQ8_0 => write!(f, "q8_0"),
            FileType::MostlyQ8_1 => write!(f, "q8_1"),
        }
    }
}

/// Each variant represents a step within the process of loading the model.
/// These can be used to report progress to the user.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum LoadProgress<'a> {
    /// The hyperparameters have been loaded from the model.
    HyperparametersLoaded(&'a Hyperparameters),
    /// The context has been created.
    ContextSize {
        /// The size of the context.
        bytes: usize,
    },
    /// A part of the model is being loaded.
    PartLoading {
        /// The path to the model part.
        file: &'a Path,
        /// The current part (0-indexed).
        current_part: usize,
        /// The number of total parts.
        total_parts: usize,
    },
    /// A tensor from the current part has been loaded.
    PartTensorLoaded {
        /// The path to the model part.
        file: &'a Path,
        /// The current tensor (0-indexed).
        current_tensor: usize,
        /// The number of total tensors.
        tensor_count: usize,
    },
    /// A model part has finished fully loading.
    PartLoaded {
        /// The path to the model part.
        file: &'a Path,
        /// The number of bytes in the part.
        byte_size: usize,
        /// The number of tensors in the part.
        tensor_count: usize,
    },
}

#[derive(Error, Debug)]
/// Errors encountered during the loading process.
pub enum LoadError {
    #[error("could not open file {path:?}")]
    /// A file failed to open.
    OpenFileFailed {
        /// The original error.
        source: std::io::Error,
        /// The path that failed.
        path: PathBuf,
    },
    #[error("no parent path for {path:?}")]
    /// There is no parent path for a given path.
    NoParentPath {
        /// The path without a parent.
        path: PathBuf,
    },
    #[error("unable to read exactly {bytes} bytes")]
    /// Reading exactly `bytes` from a file failed.
    ReadExactFailed {
        /// The original error.
        source: std::io::Error,
        /// The number of bytes that were attempted to be read.
        bytes: usize,
    },
    #[error("non-specific I/O error")]
    /// A non-specific IO error.
    Io(#[from] std::io::Error),
    #[error("could not convert bytes to a UTF-8 string")]
    /// One of the strings encountered was not valid UTF-8.
    InvalidUtf8(#[from] std::string::FromUtf8Error),
    #[error("invalid integer conversion")]
    /// One of the integers encountered could not be converted to a more appropriate type.
    InvalidIntegerConversion(#[from] std::num::TryFromIntError),
    #[error("unsupported f16_: {0}")]
    /// The `f16_` hyperparameter had an invalid value.
    UnsupportedFileType(i32),
    #[error("invalid magic number {magic:#x} for {path:?}")]
    /// An invalid magic number was encountered during the loading process.
    InvalidMagic {
        /// The path that failed.
        path: PathBuf,
        /// The magic number that was encountered.
        magic: u32,
    },
    #[error("invalid file format version {version}")]
    /// The version of the format is not supported by this version of `llama-rs`.
    InvalidFormatVersion {
        /// The format that was encountered.
        container_type: ContainerType,
        /// The version that was encountered.
        version: u32,
    },
    #[error("invalid value {ftype} for `f16` in hyperparameters")]
    /// The `f16` hyperparameter had an invalid value.
    HyperparametersF16Invalid {
        /// The format type that was encountered.
        ftype: i32,
    },
    #[error("unknown tensor `{tensor_name}` in {path:?}")]
    /// The tensor `tensor_name` was encountered during the loading of `path`, but was not seen during
    /// the model prelude.
    UnknownTensor {
        /// The name of the tensor.
        tensor_name: String,
        /// The path that failed.
        path: PathBuf,
    },
    #[error("the tensor `{tensor_name}` has the wrong size in {path:?}")]
    /// The tensor `tensor_name` did not match its expected size.
    TensorWrongSize {
        /// The name of the tensor.
        tensor_name: String,
        /// The path that failed.
        path: PathBuf,
    },
    /// The tensor `tensor_name` did not have the expected format type.
    #[error("invalid ftype {ftype} for tensor `{tensor_name}` in {path:?}")]
    UnsupportedElementType {
        /// The name of the tensor.
        tensor_name: String,
        /// The format type that was encountered.
        ftype: i32,
        /// The path that failed.
        path: PathBuf,
    },
    /// An invariant was broken.
    ///
    /// This error is not relevant unless `loader2` is being used.
    #[error("invariant broken: {invariant} in {path:?}")]
    InvariantBroken {
        /// The path that failed.
        path: PathBuf,
        /// The invariant that was broken.
        invariant: String,
    },
    /// The model could not be created.
    ///
    /// This implies that there were no tensors in the model to be loaded.
    ///
    /// This error is not relevant unless `loader2` is being used.
    #[error("could not create model from {path:?}")]
    ModelNotCreated {
        /// The path that failed.
        path: PathBuf,
    },
    /// Multiple parts of the model were found.
    ///
    /// Multi-part models are not supported. Please convert the model to a single part.
    #[error("multipart models are not supported")]
    MultipartNotSupported {
        /// The paths that were found.
        paths: Vec<PathBuf>,
    },
}
impl From<FindAllModelFilesError> for LoadError {
    fn from(value: FindAllModelFilesError) -> Self {
        match value {
            FindAllModelFilesError::NoParentPath { path } => LoadError::NoParentPath { path },
            FindAllModelFilesError::IO(err) => LoadError::Io(err),
        }
    }
}
