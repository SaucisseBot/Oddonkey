use std::pin::Pin;
use std::task::{Context, Poll};

use futures_core::Stream;

use crate::adapters::ollama::types::StreamChunk;
use crate::domain::error::OddOnkeyError;

/// A stream of token strings as they arrive from the model.
///
/// Implements `futures_core::Stream<Item = Result<String, OddOnkeyError>>`.
pub struct TokenStream {
    inner: Pin<Box<dyn Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>>,
    buffer: String,
    done: bool,
}

impl TokenStream {
    pub(crate) fn new(
        byte_stream: Pin<Box<dyn Stream<Item = Result<bytes::Bytes, reqwest::Error>> + Send>>,
    ) -> Self {
        Self {
            inner: byte_stream,
            buffer: String::new(),
            done: false,
        }
    }
}

impl Stream for TokenStream {
    type Item = Result<String, OddOnkeyError>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        if self.done {
            return Poll::Ready(None);
        }

        // Try to extract complete JSON lines from the buffer
        if let Some(newline_pos) = self.buffer.find('\n') {
            let line = self.buffer[..newline_pos].to_string();
            self.buffer = self.buffer[newline_pos + 1..].to_string();

            let line = line.trim();
            if line.is_empty() {
                cx.waker().wake_by_ref();
                return Poll::Pending;
            }

            match serde_json::from_str::<StreamChunk>(line) {
                Ok(chunk) => {
                    if chunk.done.unwrap_or(false) {
                        self.done = true;
                        return Poll::Ready(None);
                    }
                    let text = chunk.message.map(|m| m.content).unwrap_or_default();
                    return Poll::Ready(Some(Ok(text)));
                }
                Err(e) => {
                    return Poll::Ready(Some(Err(OddOnkeyError::Parse(format!(
                        "stream JSON parse: {e} – line: {line}"
                    )))));
                }
            }
        }

        // Need more data from the network
        match self.inner.as_mut().poll_next(cx) {
            Poll::Ready(Some(Ok(bytes))) => {
                self.buffer.push_str(&String::from_utf8_lossy(&bytes));
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            Poll::Ready(Some(Err(e))) => Poll::Ready(Some(Err(OddOnkeyError::Http(e)))),
            Poll::Ready(None) => {
                self.done = true;
                if !self.buffer.trim().is_empty() {
                    let line = std::mem::take(&mut self.buffer);
                    let line = line.trim();
                    match serde_json::from_str::<StreamChunk>(line) {
                        Ok(chunk) => {
                            let text = chunk.message.map(|m| m.content).unwrap_or_default();
                            if text.is_empty() {
                                Poll::Ready(None)
                            } else {
                                Poll::Ready(Some(Ok(text)))
                            }
                        }
                        Err(_) => Poll::Ready(None),
                    }
                } else {
                    Poll::Ready(None)
                }
            }
            Poll::Pending => Poll::Pending,
        }
    }
}
