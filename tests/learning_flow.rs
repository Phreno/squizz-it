use squizz_it::{
    config::AnswerMode,
    deck::Flashcard,
    game::{Session, SessionConfig, SubmitOutcome},
};

#[test]
fn engine_supports_learning_flow_without_io() {
    let cards = vec![
        Flashcard {
            key: "Q1".to_string(),
            value: "A1".to_string(),
        },
        Flashcard {
            key: "Q2".to_string(),
            value: "A2".to_string(),
        },
        Flashcard {
            key: "Q3".to_string(),
            value: "A3".to_string(),
        },
    ];

    let mut session = Session::new(
        cards,
        SessionConfig {
            answer_mode: AnswerMode::Exact,
            normalize_whitespace: true,
            shuffle_seed: Some(11),
            pre_ordered: false,
        },
    )
    .expect("valid session");

    let s1 = session.current_card().value.clone();
    assert_eq!(session.submit_answer(&s1), SubmitOutcome::AdvanceStage);

    let s2_1 = session.current_card().value.clone();
    assert_eq!(session.submit_answer(&s2_1), SubmitOutcome::AdvanceCard);
    let s2_2 = session.current_card().value.clone();
    assert_eq!(session.submit_answer("WRONG"), SubmitOutcome::Incorrect);
    assert_eq!(session.current_card().value, s2_2);
    assert_eq!(session.submit_answer(&s2_2), SubmitOutcome::AdvanceStage);

    let s3_1 = session.current_card().value.clone();
    assert_eq!(session.submit_answer(&s3_1), SubmitOutcome::AdvanceCard);
    let s3_2 = session.current_card().value.clone();
    assert_eq!(session.submit_answer(&s3_2), SubmitOutcome::AdvanceCard);
    let s3_3 = session.current_card().value.clone();
    assert_eq!(session.submit_answer(&s3_3), SubmitOutcome::RoundRestarted);
    assert_eq!(session.round(), 2);
}
