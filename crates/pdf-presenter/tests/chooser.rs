use pdf_presenter::{
    studio::{Action, Studio},
    worker::{Completion, Job, Reply, Request, ResultPacket},
};
use std::sync::mpsc::{self, Receiver, SyncSender};

fn controlled_chooser() -> (Studio, Receiver<Job>, SyncSender<Completion>) {
    let mut studio = Studio::with_program(true, None, env!("CARGO_BIN_EXE_pdf-presenter").into());
    let (requests, jobs) = mpsc::sync_channel(4);
    let (results, replies) = mpsc::sync_channel(4);
    // Replace only the chooser transport: no native dialog or renderer is opened.
    studio.dialog.tx = requests;
    studio.dialog.rx = replies;
    (studio, jobs, results)
}

#[test]
fn repeated_pdf_and_folder_requests_never_queue_another_chooser() {
    let (mut s, jobs, results) = controlled_chooser();
    s.dialog_owner = Some(12345);
    s.requested = 7;
    s.blanked = true;
    for i in 0..100 {
        s.execute(Action::Pick(i % 2 != 0));
    }
    assert!(s.dialog_busy);
    let job = jobs.try_recv().unwrap();
    assert!(matches!(
        job.request,
        Request::Pick {
            folder: false,
            owner: Some(12345)
        }
    ));
    assert!(jobs.try_recv().is_err());
    results
        .send(Completion {
            generation: 0,
            tag: 0,
            result: Ok(ResultPacket {
                reply: Reply::default(),
                data: vec![],
            }),
        })
        .unwrap();
    s.poll();
    assert!(!s.dialog_busy);
    assert_eq!(s.requested, 7);
    assert!(s.blanked);
    for _ in 0..5 {
        s.poll();
        assert!(jobs.try_recv().is_err());
    }
    s.execute(Action::Pick(true));
    assert!(s.dialog_busy);
    assert!(matches!(
        jobs.try_recv().unwrap().request,
        Request::Pick { folder: true, .. }
    ));
}

#[test]
fn chooser_selection_errors_timeout_and_submission_failure_release_the_guard() {
    let (mut s, jobs, results) = controlled_chooser();
    for result in [
        Err("Worker deadline exceeded".into()),
        Ok(ResultPacket {
            reply: Reply {
                error: Some("Dialog unavailable".into()),
                ..Default::default()
            },
            data: vec![],
        }),
        Ok(ResultPacket {
            reply: Reply {
                picked: Some("missing-test-presentation.pdf".into()),
                ..Default::default()
            },
            data: vec![],
        }),
    ] {
        s.execute(Action::Pick(false));
        assert!(s.dialog_busy);
        jobs.try_recv().unwrap();
        results
            .send(Completion {
                generation: 0,
                tag: 0,
                result,
            })
            .unwrap();
        s.poll();
        assert!(!s.dialog_busy);
        assert!(jobs.try_recv().is_err());
    }
    drop(jobs);
    s.execute(Action::Pick(false));
    assert!(!s.dialog_busy);
    assert!(s.status.contains("Could not open"));
}

#[test]
fn chooser_owner_survives_private_protocol_roundtrip() {
    for owner in [None, Some(1234567)] {
        let request = Request::Pick {
            folder: true,
            owner,
        };
        let bytes = serde_json::to_vec(&request).unwrap();
        assert!(
            matches!(serde_json::from_slice::<Request>(&bytes).unwrap(),Request::Pick{folder:true,owner:got} if got==owner)
        );
    }
}
