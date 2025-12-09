//! 协议性能基准测试 / Protocol performance benchmark
//!
//! 比较 JSON、MessagePack、Protobuf 的性能
//! Compare performance of JSON, MessagePack, and Protobuf

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use serde_json::json;
use v::plugin::protocol::*;

fn create_test_event() -> EventMessage {
    EventMessage {
        event_type: "message.incoming".to_string(),
        payload: json!({
            "message_id": "msg_1234567890",
            "from_uid": "user_123",
            "to_uid": "user_456",
            "room_id": "room_789",
            "content": {
                "type": "text",
                "text": "Hello, this is a test message with some content to make it more realistic."
            },
            "timestamp": 1234567890123i64,
            "metadata": {
                "device": "iOS",
                "version": "1.0.0",
                "ip": "192.168.1.100"
            }
        }),
        timestamp: Some(1234567890123),
        trace_id: Some("trace_abc123".to_string()),
    }
}

fn benchmark_encode(c: &mut Criterion) {
    let event = create_test_event();

    let mut group = c.benchmark_group("encode");

    // JSON 编码 / JSON encoding
    group.bench_function(BenchmarkId::new("json", "event"), |b| {
        let codec = JsonCodec;
        b.iter(|| black_box(codec.encode_event(&event).unwrap()));
    });

    // MessagePack 编码 / MessagePack encoding
    #[cfg(feature = "msgpack")]
    group.bench_function(BenchmarkId::new("msgpack", "event"), |b| {
        let codec = MessagePackCodec;
        b.iter(|| black_box(codec.encode_event(&event).unwrap()));
    });

    group.finish();
}

fn benchmark_decode(c: &mut Criterion) {
    let event = create_test_event();

    let mut group = c.benchmark_group("decode");

    // JSON 解码 / JSON decoding
    let json_codec = JsonCodec;
    let json_data = json_codec.encode_event(&event).unwrap();
    group.bench_function(BenchmarkId::new("json", "event"), |b| {
        b.iter(|| black_box(json_codec.decode_event(&json_data).unwrap()));
    });

    // MessagePack 解码 / MessagePack decoding
    #[cfg(feature = "msgpack")]
    {
        let msgpack_codec = MessagePackCodec;
        let msgpack_data = msgpack_codec.encode_event(&event).unwrap();
        group.bench_function(BenchmarkId::new("msgpack", "event"), |b| {
            b.iter(|| black_box(msgpack_codec.decode_event(&msgpack_data).unwrap()));
        });
    }

    group.finish();
}

fn benchmark_size(c: &mut Criterion) {
    let event = create_test_event();

    let mut group = c.benchmark_group("size_comparison");

    let json_codec = JsonCodec;
    let json_data = json_codec.encode_event(&event).unwrap();
    println!("JSON size: {} bytes", json_data.len());

    #[cfg(feature = "msgpack")]
    {
        let msgpack_codec = MessagePackCodec;
        let msgpack_data = msgpack_codec.encode_event(&event).unwrap();
        println!("MessagePack size: {} bytes", msgpack_data.len());
        println!(
            "Size reduction: {:.1}%",
            (1.0 - msgpack_data.len() as f64 / json_data.len() as f64) * 100.0
        );
    }

    group.finish();
}

criterion_group!(benches, benchmark_encode, benchmark_decode, benchmark_size);
criterion_main!(benches);
