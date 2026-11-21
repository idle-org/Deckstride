mod common;
use deckstride::worker::{Reply, Request, read_json, write_json};
use std::{
    io::Read,
    process::{Command, Stdio},
};
#[test]
fn worker_protocol_loads_renders_and_keeps_previous_document_after_failure() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("sample.pdf");
    common::pdf(&path);
    let cache = temp.path().join("cache");
    let mut child = Command::new(env!("CARGO_BIN_EXE_deckstride"))
        .arg("--worker")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut input = child.stdin.take().unwrap();
    let mut output = child.stdout.take().unwrap();
    write_json(
        &mut input,
        &Request::Open {
            epoch: 1,
            path: path.clone(),
            cache: cache.clone(),
        },
    )
    .unwrap();
    let reply: Reply = read_json(&mut output).unwrap();
    assert_eq!(reply.info.unwrap().names.len(), 2);
    write_json(
        &mut input,
        &Request::Open {
            epoch: 2,
            path: temp.path().join("missing.pdf"),
            cache: cache.clone(),
        },
    )
    .unwrap();
    let reply: Reply = read_json(&mut output).unwrap();
    assert!(reply.error.is_some());
    for page in [0, 1] {
        write_json(
            &mut input,
            &Request::Render {
                epoch: 1,
                path: path.clone(),
                cache: cache.clone(),
                page,
                edge: 320,
            },
        )
        .unwrap();
        let reply: Reply = read_json(&mut output).unwrap();
        assert!(reply.error.is_none(), "{:?}", reply.error);
        assert_eq!(reply.frame.unwrap().page, page);
        let mut pixels = vec![0; reply.bytes];
        output.read_exact(&mut pixels).unwrap();
        assert_eq!(pixels.len(), 320 * 180 * 4);
    }
    drop(input);
    assert!(child.wait().unwrap().success());
}
#[test]
fn protocol_rejects_oversized_and_malformed_messages() {
    let mut bytes = std::io::Cursor::new((20_000_000u32).to_le_bytes());
    assert!(read_json::<Request>(&mut bytes).is_err());
    let mut bytes = std::io::Cursor::new([2, 0, 0, 0, b'{', b'?']);
    assert!(read_json::<Request>(&mut bytes).is_err());
    let mut bytes = vec![];
    write_json(&mut bytes, &Request::SleepTest(1)).unwrap();
    assert!(matches!(
        read_json::<Request>(&mut bytes.as_slice()).unwrap(),
        Request::SleepTest(1)
    ));
}

#[test]
fn supervisor_kills_a_stuck_worker_and_recovers() {
    use std::time::{Duration, Instant};
    let bridge = deckstride::worker::Bridge::with_program(
        Duration::from_millis(300),
        env!("CARGO_BIN_EXE_deckstride").into(),
    );
    let start = Instant::now();
    assert!(bridge.submit(7, 1, Request::SleepTest(5000)));
    let c = bridge.rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert_eq!((c.generation, c.tag), (7, 1));
    assert!(c.result.err().unwrap().contains("deadline"));
    assert!(start.elapsed() < Duration::from_secs(3));
    assert!(bridge.submit(8, 2, Request::SleepTest(1)));
    let c = bridge.rx.recv_timeout(Duration::from_secs(5)).unwrap();
    assert!(c.result.is_ok());
    assert_eq!((c.generation, c.tag), (8, 2));
}

#[test]
fn output_diagnostics_are_distinct_from_explicit_blanking() {
    use deckstride::output::Ack;
    let old: Ack =
        serde_json::from_str(r#"{"sequence":0,"page":null,"status":"host restarted","awake":""}"#)
            .unwrap();
    assert!(!old.request_blank);
    let explicit = Ack {
        request_blank: true,
        ..Default::default()
    };
    let restored: Ack = serde_json::from_str(&serde_json::to_string(&explicit).unwrap()).unwrap();
    assert!(restored.request_blank);
}
