// Copyright Amazon.com, Inc. or its affiliates. All Rights Reserved.
// SPDX-License-Identifier: Apache-2.0

#[cfg(all(feature = "fips", target_os = "windows"))]
std::compile_error!("feature `fips` is not supported on windows");

#[macro_use]
mod negotiated;
#[macro_use]
mod header_key;

mod aead;
mod cipher_suite;
mod iv;

#[doc(hidden)]
pub use aws_lc_rs::{
    aead as aws_lc_aead, aead::MAX_TAG_LEN, constant_time, digest, hkdf, hkdf::Prk, hmac,
};

#[derive(Clone)]
pub struct SecretPair {
    pub server: Prk,
    pub client: Prk,
}

pub mod handshake;
pub mod initial;
pub mod one_rtt;
pub mod retry;
pub mod zero_rtt;

#[derive(Clone, Copy, Debug, Default)]
pub struct Suite;

impl s2n_quic_core::crypto::CryptoSuite for Suite {
    type HandshakeKey = handshake::HandshakeKey;
    type HandshakeHeaderKey = handshake::HandshakeHeaderKey;
    type InitialKey = initial::InitialKey;
    type InitialHeaderKey = initial::InitialHeaderKey;
    type OneRttKey = one_rtt::OneRttKey;
    type OneRttHeaderKey = one_rtt::OneRttHeaderKey;
    type ZeroRttKey = zero_rtt::ZeroRttKey;
    type ZeroRttHeaderKey = zero_rtt::ZeroRttHeaderKey;
    type RetryKey = retry::RetryKey;
}
