// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

use crate::{crypto::decrypt, packet::stream};
use s2n_quic_core::{buffer, frame};

#[derive(Clone, Copy, Debug, thiserror::Error)]
pub enum Error {
    #[error("could not decode packet")]
    Decode,
    #[error("could not decrypt packet")]
    Decrypt,
    #[error("packet has already been processed")]
    Duplicate,
    #[error("the packet was for another stream ({actual}) but was handled by {expected}")]
    StreamMismatch {
        expected: stream::Id,
        actual: stream::Id,
    },
    #[error("the stream expected in-order delivery of {expected} but got {actual}")]
    OutOfOrder { expected: u64, actual: u64 },
    #[error("the peer exceeded the max data window")]
    MaxDataExceeded,
    #[error("invalid fin")]
    InvalidFin,
    #[error("out of range")]
    OutOfRange,
    #[error("unexpected retransmission packet")]
    UnexpectedRetransmission,
    #[error("the transport has been truncated without authentication")]
    TruncatedTransport,
    #[error("the receiver idle timer expired")]
    IdleTimeout,
    #[error("the crypto key has been replayed and is invalid")]
    KeyReplayPrevented,
    #[error("the crypto key has been potentially replayed (gap: {gap:?}) and is invalid")]
    KeyReplayMaybePrevented { gap: Option<u64> },
    #[error("application error: {error}")]
    ApplicationError {
        error: s2n_quic_core::application::Error,
    },
}

impl From<decrypt::Error> for Error {
    fn from(value: decrypt::Error) -> Self {
        match value {
            decrypt::Error::ReplayDefinitelyDetected => Self::KeyReplayPrevented,
            decrypt::Error::ReplayPotentiallyDetected { gap } => {
                Self::KeyReplayMaybePrevented { gap }
            }
            decrypt::Error::InvalidTag => Self::Decrypt,
        }
    }
}

impl Error {
    #[inline]
    pub(super) fn is_fatal(&self, features: &super::TransportFeatures) -> bool {
        // if the transport is a stream then any error we encounter is fatal, since the stream is
        // now likely corrupted
        if features.is_stream() {
            return true;
        }

        !matches!(
            self,
            Self::Decode | Self::Decrypt | Self::Duplicate | Self::StreamMismatch { .. }
        )
    }

    #[inline]
    pub(super) fn connection_close(&self) -> Option<frame::ConnectionClose<'static>> {
        use s2n_quic_core::transport;
        match self {
            Error::Decode
            | Error::Decrypt
            | Error::Duplicate
            | Error::StreamMismatch { .. }
            | Error::UnexpectedRetransmission => {
                // return protocol violation for the errors that are only fatal for reliable
                // transports
                Some(transport::Error::PROTOCOL_VIOLATION.into())
            }
            Error::IdleTimeout => None,
            Error::MaxDataExceeded => Some(transport::Error::FLOW_CONTROL_ERROR.into()),
            Error::InvalidFin | Error::TruncatedTransport => {
                Some(transport::Error::FINAL_SIZE_ERROR.into())
            }
            Error::OutOfOrder { .. } => Some(transport::Error::STREAM_STATE_ERROR.into()),
            Error::OutOfRange => Some(transport::Error::STREAM_LIMIT_ERROR.into()),
            // we don't have working crypto keys so we can't respond
            Error::KeyReplayPrevented | Error::KeyReplayMaybePrevented { .. } => None,
            Error::ApplicationError { error } => Some((*error).into()),
        }
    }
}

impl From<buffer::Error<Error>> for Error {
    #[inline]
    fn from(value: buffer::Error<Error>) -> Self {
        match value {
            buffer::Error::OutOfRange => Self::OutOfRange,
            buffer::Error::InvalidFin => Self::InvalidFin,
            buffer::Error::ReaderError(error) => error,
        }
    }
}

impl From<Error> for std::io::Error {
    #[inline]
    fn from(error: Error) -> Self {
        Self::new(error.into(), error)
    }
}

impl From<Error> for std::io::ErrorKind {
    #[inline]
    fn from(error: Error) -> Self {
        use std::io::ErrorKind;
        match error {
            Error::Decode => ErrorKind::InvalidData,
            Error::Decrypt => ErrorKind::InvalidData,
            Error::Duplicate => ErrorKind::InvalidData,
            Error::StreamMismatch { .. } => ErrorKind::InvalidData,
            Error::MaxDataExceeded => ErrorKind::ConnectionAborted,
            Error::InvalidFin => ErrorKind::InvalidData,
            Error::TruncatedTransport => ErrorKind::UnexpectedEof,
            Error::OutOfRange => ErrorKind::ConnectionAborted,
            Error::OutOfOrder { .. } => ErrorKind::InvalidData,
            Error::UnexpectedRetransmission { .. } => ErrorKind::InvalidData,
            Error::IdleTimeout => ErrorKind::TimedOut,
            Error::KeyReplayPrevented => ErrorKind::PermissionDenied,
            Error::KeyReplayMaybePrevented { .. } => ErrorKind::PermissionDenied,
            Error::ApplicationError { .. } => ErrorKind::ConnectionReset,
        }
    }
}
