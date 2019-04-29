// Copyright 2019 Parity Technologies (UK) Ltd.
// This file is part of Substrate.

// Substrate is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// Substrate is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with Substrate.  If not, see <http://www.gnu.org/licenses/>.

//! Offchain workers types

/// Opaque type for offchain http requests.
#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct HttpRequestId(pub u16);

/// Status of the HTTP request
#[derive(Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "std", derive(Debug))]
pub enum HttpRequestStatus {
	/// Deadline was reached why we waited for this request to finish.
	DeadlineReached,
	/// Request timed out.
	Timeout,
	/// Request status of this ID is not known.
	Unknown,
	/// The request is finished with given status code.
	Finished(u16),
}

impl HttpRequestStatus {
	/// Parse u16 as `RequestStatus`.
	///
	/// The first hundred of codes indicate internal states.
	/// The rest are http response status codes.
	pub fn from_u16(status: u16) -> Option<Self> {
		match status {
			0 => Some(HttpRequestStatus::Unknown),
			10 => Some(HttpRequestStatus::DeadlineReached),
			20 => Some(HttpRequestStatus::Timeout),
			100...999 => Some(HttpRequestStatus::Finished(status)),
			_ => None,
		}
	}
}

/// Opaque timestamp type
#[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Timestamp(u64);

/// Duration type
#[derive(Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
#[cfg_attr(feature = "std", derive(Debug))]
pub struct Duration(u64);

impl Duration {
	/// Create new duration representing given number of milliseconds.
	pub fn from_millis(millis: u64) -> Self {
		Duration(millis)
	}

	/// Returns number of milliseconds this Duration represents.
	pub fn millis(&self) -> u64 {
		self.0
	}
}

impl Timestamp {
	/// Creates new `Timestamp` given unix timestamp in miliseconds.
	pub fn from_unix_millis(millis: u64) -> Self {
		Timestamp(millis)
	}

	/// Increase the timestamp by given `Duration`.
	pub fn add(&self, duration: Duration) -> Timestamp {
		Timestamp(self.0.saturating_add(duration.0))
	}

	/// Decrease the timestamp by given `Duration`
	pub fn sub(&self, duration: Duration) -> Timestamp {
		Timestamp(self.0.saturating_sub(duration.0))
	}

	/// Returns a saturated difference (Duration) between two Timestamps.
	pub fn diff(&self, other: &Self) -> Duration {
		Duration(self.0.saturating_sub(other.0))
	}

	/// Return number of milliseconds since UNIX epoch.
	pub fn unix_millis(&self) -> u64 {
		self.0
	}
}

/// An extended externalities for offchain workers.
pub trait Externalities {
	/// Submit extrinsic.
	///
	/// The extrinsic will either go to the pool (signed)
	/// or to the next produced block (inherent).
	/// Returns an error in case the API is not available.
	fn submit_extrinsic(&mut self, extrinsic: Vec<u8>) -> Result<(), ()>;

	/// Returns current UNIX timestamp in Milliseconds
	///
	/// Returns an error in case the API is not available.
	fn timestamp(&mut self) -> Result<u64, ()>;

	/// Initiaties a http request given HTTP verb and the URL.
	///
	/// Meta is a future-reserved field containing additional, parity-codec encoded parameters.
	/// Returns the id of newly started request.
	fn http_request_start(
		&mut self,
		method: &str,
		uri: &str,
		meta: &[u8]
	) -> Result<HttpRequestId, ()>;

	/// Append header to the request.
	fn http_request_add_header(
		&mut self,
		request_id: HttpRequestId,
		name: &str,
		value: &str
	) -> Result<(), ()>;

	/// Write a chunk of request body.
	///
	/// Writing an empty chunks finalises the request.
	/// Passing `None` as deadline blocks forever.
	///
	/// Returns an error in case deadline is reached or the chunk couldn't be written.
	fn http_request_write_body(
		&mut self,
		request_id: HttpRequestId,
		chunk: &[u8],
		deadline: Option<Timestamp>
	) -> Result<(), ()>;

	/// Block and wait for the responses for given requests.
	///
	/// Returns a vector of request statuses (the len is the same as ids).
	/// Note that if deadline is not provided the method will block indefinitely,
	/// otherwise unready responses will produce `WaitTimeout` status.
	///
	/// Passing `None` as deadline blocks forever.
	fn http_response_wait(
		&mut self,
		ids: &[HttpRequestId],
		deadline: Option<Timestamp>
	) -> Vec<HttpRequestStatus>;

	/// Read all response headers.
	///
	/// Resturns a vector of pairs `(HeaderKey, HeaderValue)`.
	fn http_response_headers(
		&mut self,
		request_id: HttpRequestId
	) -> Vec<(Vec<u8>, Vec<u8>)>;

	/// Read a chunk of body response to given buffer.
	///
	/// Returns the number of bytes written or an error in case a deadline
	/// is reached or server closed the connection.
	/// Passing `None` as a deadline blocks forever.
	fn http_response_read_body(
		&mut self,
		request_id: HttpRequestId,
		buffer: &mut [u8],
		deadline: Option<Timestamp>
	) -> Result<usize, ()>;

}
impl<T: Externalities + ?Sized> Externalities for Box<T> {
	fn submit_extrinsic(&mut self, ex: Vec<u8>) -> Result<(), ()> {
		(&mut **self).submit_extrinsic(ex)
	}

	fn timestamp(&mut self) -> Result<u64, ()> {
		(&mut **self).timestamp()
	}

	fn http_request_start(&mut self, method: &str, uri: &str, meta: &[u8]) -> Result<HttpRequestId, ()> {
		(&mut **self).http_request_start(method, uri, meta)
	}

	fn http_request_add_header(&mut self, request_id: HttpRequestId, name: &str, value: &str) -> Result<(), ()> {
		(&mut **self).http_request_add_header(request_id, name, value)
	}

	fn http_request_write_body(
		&mut self,
		request_id: HttpRequestId,
		chunk: &[u8],
		deadline: Option<Timestamp>
	) -> Result<(), ()> {
		(&mut **self).http_request_write_body(request_id, chunk, deadline)
	}

	fn http_response_wait(&mut self, ids: &[HttpRequestId], deadline: Option<Timestamp>) -> Vec<HttpRequestStatus> {
		(&mut **self).http_response_wait(ids, deadline)
	}

	fn http_response_headers(&mut self, request_id: HttpRequestId) -> Vec<(Vec<u8>, Vec<u8>)> {
		(&mut **self).http_response_headers(request_id)
	}

	fn http_response_read_body(
		&mut self,
		request_id: HttpRequestId,
		buffer: &mut [u8],
		deadline: Option<Timestamp>
	) -> Result<usize, ()> {
		(&mut **self).http_response_read_body(request_id, buffer, deadline)
	}
}


#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn timestamp_ops() {
		let t = Timestamp(5);
		assert_eq!(t.add(Duration::from_millis(10)), Timestamp(15));
		assert_eq!(t.sub(Duration::from_millis(10)), Timestamp(0));
		assert_eq!(t.diff(&Timestamp(3)), Duration(2));
	}
}