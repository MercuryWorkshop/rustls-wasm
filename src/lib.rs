use std::{pin::Pin, sync::Arc};

use combine_rw::CombineRW;
use futures_rustls::{
	rustls::{ClientConfig, RootCertStore},
	TlsConnector,
};
use futures_util::{
	AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, SinkExt, StreamExt, TryStreamExt,
};
use js_sys::{Array, ArrayBuffer, Object, Uint8Array};
use thiserror::Error;
use wasm_bindgen::prelude::*;
use web_sys::{ReadableStream, WritableStream};
use webpki_roots::TLS_SERVER_ROOTS;

mod combine_rw;

#[derive(Debug, Error)]
pub enum RustlsWasmError {
	#[error("{0}")]
	Js(String),
	#[error("Invalid DNS name: {0:?}")]
	InvalidDnsName(#[from] futures_rustls::pki_types::InvalidDnsNameError),
	#[error("IO: {0:?}")]
	Io(#[from] std::io::Error),

	#[error("Invalid payload")]
	InvalidPayload,
}

impl From<RustlsWasmError> for JsValue {
	fn from(value: RustlsWasmError) -> Self {
		JsError::from(value).into()
	}
}

impl From<RustlsWasmError> for std::io::Error {
	fn from(value: RustlsWasmError) -> Self {
		std::io::Error::other(value)
	}
}

impl From<js_sys::Error> for RustlsWasmError {
	fn from(value: js_sys::Error) -> Self {
		Self::Js(format!("{:?}", value))
	}
}

impl From<JsValue> for RustlsWasmError {
	fn from(value: JsValue) -> Self {
		Self::Js(format!("{:?}", value))
	}
}

fn jsval_to_vec(val: JsValue) -> Result<Vec<u8>, RustlsWasmError> {
	if let Some(str) = val.as_string() {
		Ok(str.into())
	} else if let Some(arr) = val.dyn_ref::<ArrayBuffer>() {
		Ok(Uint8Array::new(arr).to_vec())
	} else {
		Err(RustlsWasmError::InvalidPayload)
	}
}

fn create_obj(read: ReadableStream, write: WritableStream) -> Object {
	Object::from_entries(&Array::of2(
		&Array::of2(&"read".into(), &read),
		&Array::of2(&"write".into(), &write),
	))
	.unwrap()
}

fn attempt_byob(
	read: ReadableStream,
	write: WritableStream,
) -> Result<(impl AsyncRead, impl AsyncWrite), RustlsWasmError> {
	let read = match wasm_streams::ReadableStream::from_raw(read).try_into_async_read() {
		Ok(read) => Box::pin(read) as Pin<Box<dyn AsyncRead>>,
		Err((_, read)) => Box::pin(
			read.try_into_stream()
				.map_err(|(err, _)| err)?
				.map(|x| Ok(jsval_to_vec(x.map_err(RustlsWasmError::from)?)?))
				.into_async_read(),
		) as Pin<Box<dyn AsyncRead>>,
	};

	let write = wasm_streams::WritableStream::from_raw(write)
		.try_into_async_write()
		.map_err(|(err, _)| err)?;

	Ok((read, write))
}

#[wasm_bindgen]
pub async fn connect_tls(
	read: ReadableStream,
	write: WritableStream,
	host: String,
) -> Result<JsValue, RustlsWasmError> {
	let (read, write) = attempt_byob(read, write)?;

	// TODO: add ClientConfig struct
	let config = Arc::new(
		ClientConfig::builder()
			.with_root_certificates(RootCertStore::from_iter(TLS_SERVER_ROOTS.iter().cloned()))
			.with_no_client_auth(),
	);

	let connector = TlsConnector::from(config);
	let connected = connector
		.connect(host.try_into()?, CombineRW::new(read, write))
		.await?;

	let (read, write) = connected.split();

	let read = wasm_streams::ReadableStream::from_async_read(read, 1024).into_raw();
	let write = wasm_streams::WritableStream::from_sink(
		write
			.into_sink()
			.with(|x| async { jsval_to_vec(x) })
			.sink_map_err(JsValue::from),
	)
	.into_raw();

	Ok(create_obj(read, write).into())
}
