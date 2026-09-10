//! In-process fake CLI for the agent-SDK tests: no subprocess, no network.
//!
//! The SDK and the fake speak newline-delimited JSON over a `tokio::io::duplex`
//! pair — the SDK writes envelopes to its stdin, which the fake reads, and reads
//! responses from its stdout, which the fake writes. A script is an async
//! closure running one canned read/assert/respond sequence.
#![allow(dead_code)]

use std::future::Future;

use intelligence_claude_agent_sdk::{ProcessSpawnOptions, Result, SpawnedClaudeProcess};
use serde_json::Value;
use tokio::io::{
    AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader, ReadHalf, WriteHalf,
};

/// Reader half the fake script uses to consume JSONL the SDK wrote to its stdin.
pub type FakeReader = BufReader<ReadHalf<tokio::io::DuplexStream>>;
/// Writer half the fake script uses to emit JSONL the SDK reads from its stdout.
pub type FakeWriter = WriteHalf<tokio::io::DuplexStream>;

/// `None` means EOF: the SDK closed its stdin.
pub async fn read_json_line(reader: &mut FakeReader) -> Option<Value> {
    let mut line = String::new();
    let read = reader.read_line(&mut line).await.expect("read fake stdin");
    if read == 0 {
        return None;
    }
    let trimmed = line.trim_end_matches('\n');
    if trimmed.is_empty() {
        return Some(Value::Null);
    }
    Some(serde_json::from_str(trimmed).expect("fake stdin line is valid JSON"))
}

/// Panics at EOF, for the steps where a line must be present.
pub async fn expect_json_line(reader: &mut FakeReader) -> Value {
    read_json_line(reader)
        .await
        .expect("expected a JSON line from the SDK but hit EOF")
}

pub async fn write_json_line(writer: &mut FakeWriter, value: &Value) {
    let mut bytes = serde_json::to_vec(value).expect("serialize fake stdout line");
    bytes.push(b'\n');
    writer.write_all(&bytes).await.expect("write fake stdout");
    writer.flush().await.expect("flush fake stdout");
}

/// `ProcessSpawnOptions` reaches the script unchanged, so a script can assert
/// on the command, args and env the SDK would have spawned with.
pub fn fake_cli<F, Fut>(script: F) -> impl Fn(ProcessSpawnOptions) -> ProcessFuture
where
    F: Fn(FakeReader, FakeWriter, ProcessSpawnOptions) -> Fut + Send + Sync + Clone + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    move |options: ProcessSpawnOptions| {
        let script = script.clone();
        Box::pin(async move {
            // sdk_stdin: SDK writes here, fake reads it.
            let (sdk_stdin_sdk_side, sdk_stdin_fake_side) = tokio::io::duplex(64 * 1024);
            // sdk_stdout: fake writes here, SDK reads it.
            let (sdk_stdout_sdk_side, sdk_stdout_fake_side) = tokio::io::duplex(64 * 1024);

            let (fake_reader_raw, _drop_writer) = tokio::io::split(sdk_stdin_fake_side);
            let (_drop_reader, fake_writer) = tokio::io::split(sdk_stdout_fake_side);
            let fake_reader = BufReader::new(fake_reader_raw);

            let handle = tokio::spawn(async move {
                script(fake_reader, fake_writer, options).await;
            });

            let wait = async move {
                let _ = handle.await;
                Ok(())
            };
            Ok(SpawnedClaudeProcess::new(
                sdk_stdin_sdk_side,
                sdk_stdout_sdk_side,
                wait,
                || {},
            ))
        }) as ProcessFuture
    }
}

pub type ProcessFuture =
    std::pin::Pin<Box<dyn Future<Output = Result<SpawnedClaudeProcess>> + Send>>;

// Keep AsyncRead/AsyncWrite in scope for callers that build custom adapters.
const _: fn() = || {
    fn _assert_read<T: AsyncRead>() {}
    fn _assert_write<T: AsyncWrite>() {}
};
