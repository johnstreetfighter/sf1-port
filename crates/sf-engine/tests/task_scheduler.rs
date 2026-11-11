//! Ticket #14: the task scheduler decodes each fixture's task positions
//! from the tasks' stacks, and refuses a position whose task body is not
//! translated (ticket #29 added the intro loop). (Every scene test runs its frames through the scheduler and
//! compares the slots' state and sleep bytes with the reference after every
//! frame.)
//!
//! Consumes local-only reference captures; skips on a clean checkout.

use std::path::Path;

use sf_engine::tasks::Scheduler;

const CAPTURES: &str = "../../tools/reference/local/captures";

fn work(capture: &str, frame: u32) -> Option<Vec<u8>> {
    std::fs::read(
        Path::new(CAPTURES)
            .join(capture)
            .join(format!("ram_{frame:06}_work.bin")),
    )
    .ok()
}

#[test]
fn fight_fixtures_decode_to_translated_task_positions() {
    let cases = [
        (
            "move-run-a",
            2450,
            [
                (0, 0x3f7ac),
                (1, 0x3eafa),
                (2, 0x3f7d6),
                (3, 0x3f3a2),
                (7, 0x48270),
            ],
        ),
        (
            "cpu-01-run-a",
            2389,
            [
                (0, 0x3f7ac),
                (1, 0x3ea3e),
                (2, 0x3f7d6),
                (3, 0x3f3a2),
                (7, 0x48270),
            ],
        ),
        (
            "round2-run-a",
            3900,
            [
                (0, 0x3f7ac),
                (1, 0x3eafa),
                (2, 0x3f7d6),
                (3, 0x3f3a2),
                (7, 0x48270),
            ],
        ),
    ];
    for (capture, frame, want) in cases {
        let Some(w) = work(capture, frame) else {
            eprintln!("skipping: {capture} not present");
            return;
        };
        let s = Scheduler::from_fixture(&w).unwrap_or_else(|e| panic!("{capture} {frame}: {e}"));
        assert_eq!(s.positions(), want, "{capture} {frame}");
    }
}

#[test]
fn an_untranslated_task_position_is_refused() {
    // The versus fixture with slot 1's resume address (on its stack, past
    // the sleep wrapper's saved registers and return) moved to an address
    // whose task body is not translated.
    let Some(mut w) = work("move-run-a", 2450) else {
        eprintln!("skipping: move-run-a not present");
        return;
    };
    let usp = u32::from_be_bytes(w[0x16 + 8..0x16 + 12].try_into().unwrap());
    let at = (usp + 64 - 0xff8000) as usize;
    w[at..at + 4].copy_from_slice(&0x3e000u32.to_be_bytes());
    let err = Scheduler::from_fixture(&w).err().expect("refused");
    assert!(err.contains("slot 1") && err.contains("0x3e000"), "{err}");
}

#[test]
fn the_idle_intro_fixture_decodes_to_the_translated_intro_loop() {
    let Some(w) = work("idle-run-a", 2280) else {
        eprintln!("skipping: idle-run-a not present");
        return;
    };
    let s = Scheduler::from_fixture(&w).unwrap();
    assert!(
        s.positions().contains(&(1, 0x3ea30)),
        "{:x?}",
        s.positions()
    );
}

#[test]
fn attract_794_fixture_and_loop_positions_decode_and_refuse_invalid_states() {
    let Some(w) = work("fight-run-a", 794) else {
        eprintln!("skipping: fight-run-a not present");
        return;
    };
    assert_eq!(
        u32::from_be_bytes(w[0xf20..0xf24].try_into().unwrap()),
        0x3f3d8,
        "slot 3 attract continuation at 0xff8f20"
    );
    let s = Scheduler::from_fixture(&w).expect("794 fixture decodes");
    assert_eq!(
        s.positions(),
        vec![(0, 0x3f7ac), (1, 0x3ccec), (3, 0x3f3a2), (7, 0x4819e)],
    );

    let slot1_usp = u32::from_be_bytes(w[0x16 + 8..0x16 + 12].try_into().unwrap());
    let resume_off = (slot1_usp + 64 - 0xff8000) as usize;
    let d1_off = (slot1_usp + 6 - 0xff8000) as usize;
    let d2_off = (slot1_usp + 10 - 0xff8000) as usize;
    let d3_off = (slot1_usp + 14 - 0xff8000) as usize;
    let a2_off = (slot1_usp + 40 - 0xff8000) as usize;

    // Scroll loop (0x3cd50, D1 in 0..10) and title-block loop (0x3cd86, D1 in 0..3).
    for (resume, max_d1, a2) in [(0x3cd50u32, 9u16, 0x3cde0u32), (0x3cd86, 2, 0x3cdc8)] {
        let mut w_loop = w.clone();
        w_loop[resume_off..resume_off + 4].copy_from_slice(&resume.to_be_bytes());
        w_loop[d1_off..d1_off + 2].copy_from_slice(&max_d1.to_be_bytes());
        w_loop[d2_off..d2_off + 2].copy_from_slice(&0u16.to_be_bytes());
        w_loop[d3_off..d3_off + 2].copy_from_slice(&(max_d1 << 3).to_be_bytes());
        w_loop[a2_off..a2_off + 4].copy_from_slice(&a2.to_be_bytes());
        let s = Scheduler::from_fixture(&w_loop).expect("valid loop state decodes");
        assert!(s.positions().contains(&(1, resume)));
        w_loop[d1_off..d1_off + 2].copy_from_slice(&(max_d1 + 1).to_be_bytes());
        assert!(
            Scheduler::from_fixture(&w_loop).is_err(),
            "out-of-range D1 refused"
        );
    }

    // Logo hold (0x3cda8) and second half of 300-frame wait (0x3cdb8).
    for addr in [0x3cda8u32, 0x3cdb8] {
        let mut w_hold = w.clone();
        w_hold[resume_off..resume_off + 4].copy_from_slice(&addr.to_be_bytes());
        let s = Scheduler::from_fixture(&w_hold).expect("hold position decodes");
        assert!(s.positions().contains(&(1, addr)));
    }

    // Nested sleep 0x3b582 / 0x3b588: caller at usp+68 must be 0x3cdb8.
    let mut w_nested = w.clone();
    w_nested[resume_off..resume_off + 4].copy_from_slice(&0x3b588u32.to_be_bytes());
    w_nested[resume_off + 4..resume_off + 8].copy_from_slice(&0x3cdb8u32.to_be_bytes());
    let s = Scheduler::from_fixture(&w_nested).expect("0x3b588 -> 0x3cdb8 decodes");
    assert!(s.positions().contains(&(1, 0x3b588)));
    w_nested[resume_off + 4..resume_off + 8].copy_from_slice(&0x3ce00u32.to_be_bytes());
    assert!(
        Scheduler::from_fixture(&w_nested).is_err(),
        "0x3b588 with wrong caller refused"
    );

    // Attract ranking wait 0x42736 (ticket #56, F16): caller at usp+68 must be 0x3ce54.
    let mut w_ranking = w.clone();
    w_ranking[resume_off..resume_off + 4].copy_from_slice(&0x42736u32.to_be_bytes());
    w_ranking[resume_off + 4..resume_off + 8].copy_from_slice(&0x3ce54u32.to_be_bytes());
    let s = Scheduler::from_fixture(&w_ranking).expect("0x42736 -> 0x3ce54 decodes");
    assert!(s.positions().contains(&(1, 0x42736)));
    w_ranking[resume_off + 4..resume_off + 8].copy_from_slice(&0x3df1eu32.to_be_bytes());
    assert!(
        Scheduler::from_fixture(&w_ranking).is_err(),
        "0x42736 with wrong caller refused"
    );

    // Slot 3 continuation at 0xff8f20 must be one of 0x3f3be / 0x3f3d8 / 0x3f402 in attract mode.
    let mut w_watch = w.clone();
    w_watch[0xf20..0xf24].copy_from_slice(&0x3f400u32.to_be_bytes());
    assert!(
        Scheduler::from_fixture(&w_watch).is_err(),
        "untranslated 0xff8f20 refused"
    );
}

#[test]
fn prefight_1865_fixture_decodes_to_slot1_0x3e59c_and_slot2_0x3f7d6() {
    let Some(w) = work("prefight-01-run-a", 1865) else {
        eprintln!("skipping: prefight-01-run-a not present");
        return;
    };
    let s = Scheduler::from_fixture(&w).expect("1865 fixture decodes");
    assert_eq!(
        s.positions(),
        vec![
            (0, 0x3f7ac),
            (1, 0x3e59c),
            (2, 0x3f7d6),
            (3, 0x3f3a2),
            (7, 0x4819e),
        ]
    );
}
