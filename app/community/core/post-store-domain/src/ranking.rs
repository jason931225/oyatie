//! Deterministic integer-only hot/controversy ranking: no floats, saturating throughout.

use crate::{VoteKind, VoteLedger};

/// Recency weight: one day in seconds; mirrors `RECENCY_WEIGHT` in `feed_ranking.rs`.
pub const RECENCY_WEIGHT: u64 = 86_400;

impl VoteLedger {
    /// Hot score: net tally blended with an age-decay recency term; saturating, no panics.
    pub fn hot_score(&self, created_at: u64, now: u64) -> i64 {
        let age_secs = now.saturating_sub(created_at);
        let recency_term = RECENCY_WEIGHT.saturating_sub(age_secs.min(RECENCY_WEIGHT));
        self.tally().saturating_add(recency_term as i64)
    }

    /// Controversy score `min(up, down) * (up + down)`, saturating; 0 when one-directional.
    pub fn controversy_score(&self) -> u64 {
        let (up, down) = self
            .receipts
            .value
            .iter()
            .fold((0u64, 0u64), |(u, d), r| match r.kind {
                VoteKind::Up => (u.saturating_add(1), d),
                VoteKind::Down => (u, d.saturating_add(1)),
            });
        up.min(down).saturating_mul(up.saturating_add(down))
    }
}

/// Rank entries by `hot_score` descending, ascending `post_id` tiebreak.
/// Entries whose `post_id` is empty or blank are excluded.
pub fn rank_posts(entries: &[(&str, &VoteLedger, u64)], now: u64) -> Vec<String> {
    let mut ranked: Vec<(&str, i64)> = entries
        .iter()
        .filter(|(post_id, _, _)| !post_id.trim().is_empty())
        .map(|(post_id, ledger, created_at)| (*post_id, ledger.hot_score(*created_at, now)))
        .collect();

    ranked.sort_by(|(id_a, score_a), (id_b, score_b)| {
        score_b.cmp(score_a).then_with(|| id_a.cmp(id_b))
    });

    ranked.into_iter().map(|(id, _)| id.to_owned()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{CommunityAuthor, CommunityMode, CommunityPost, VoteKind, VoteReceipt};

    fn author() -> CommunityAuthor {
        CommunityAuthor::new("anon".into(), "user:real".into(), Some("policy".into())).unwrap()
    }

    fn post(post_id: &str) -> CommunityPost {
        CommunityPost::new(
            post_id.into(),
            "t".into(),
            "tenant".into(),
            CommunityMode::Teamblind,
            author(),
            "body".into(),
            "retain".into(),
        )
        .unwrap()
    }

    fn ledger_with_votes(post_id: &str, up: u32, down: u32) -> VoteLedger {
        let p = post(post_id);
        let mut l = VoteLedger::new(&p);
        for i in 0..up {
            l.cast(
                VoteReceipt {
                    vote_id: format!("u{i}"),
                    voter_ref: format!("voter:up:{i}"),
                    post_id: post_id.into(),
                    kind: VoteKind::Up,
                },
                &p,
            )
            .unwrap();
        }
        for i in 0..down {
            l.cast(
                VoteReceipt {
                    vote_id: format!("d{i}"),
                    voter_ref: format!("voter:down:{i}"),
                    post_id: post_id.into(),
                    kind: VoteKind::Down,
                },
                &p,
            )
            .unwrap();
        }
        l
    }

    #[test]
    fn hot_score_net_upvote_raises_score() {
        let now = 1_000_000u64;
        let created_at = now - 3600; // 1 hour old

        let no_votes = ledger_with_votes("p", 0, 0);
        let upvoted = ledger_with_votes("p", 5, 0);

        assert!(
            upvoted.hot_score(created_at, now) > no_votes.hot_score(created_at, now),
            "net upvotes must raise hot_score"
        );
    }

    #[test]
    fn hot_score_decay_monotonic() {
        let now = 1_000_000u64;
        let l = ledger_with_votes("p", 3, 1); // tally = 2

        let fresh = l.hot_score(now - 60, now); // 1 min old
        let hour_old = l.hot_score(now - 3600, now); // 1 hour old
        let day_old = l.hot_score(now - 86_400, now); // exactly 1 day old

        assert!(fresh >= hour_old, "fresher post must score >= hour-old");
        assert!(hour_old >= day_old, "hour-old post must score >= day-old");
    }

    #[test]
    fn hot_score_decay_floor() {
        let now = 1_000_000u64;
        let l = ledger_with_votes("p", 3, 1); // tally = 2

        let ancient_score = l.hot_score(now - 200_000, now);
        assert_eq!(
            ancient_score,
            l.tally(),
            "ancient post hot_score must equal tally() only"
        );
    }

    #[test]
    fn hot_score_empty_ledger_at_zero_age() {
        let now = 1_000_000u64;
        let l = ledger_with_votes("p", 0, 0);
        assert_eq!(
            l.hot_score(now, now),
            RECENCY_WEIGHT as i64,
            "empty ledger at age 0 must equal RECENCY_WEIGHT"
        );
    }

    #[test]
    fn controversy_score_zero_when_one_directional() {
        let all_up = ledger_with_votes("p", 5, 0);
        let all_down = ledger_with_votes("p", 0, 5);
        assert_eq!(all_up.controversy_score(), 0, "all-up must score 0");
        assert_eq!(all_down.controversy_score(), 0, "all-down must score 0");
    }

    #[test]
    fn controversy_score_maximal_at_equal_split() {
        let equal = ledger_with_votes("p", 2, 2); // min=2, total=4 → 2*4=8
        let lopsided = ledger_with_votes("q", 3, 1); // min=1, total=4 → 1*4=4
        assert!(
            equal.controversy_score() > lopsided.controversy_score(),
            "equal split must score higher than lopsided"
        );
    }

    #[test]
    fn controversy_score_symmetric() {
        let up_heavy = ledger_with_votes("p", 7, 3);
        let down_heavy = ledger_with_votes("q", 3, 7);
        assert_eq!(
            up_heavy.controversy_score(),
            down_heavy.controversy_score(),
            "swapping up/down must give identical controversy_score"
        );
    }

    #[test]
    fn rank_posts_deterministic_stable_tiebreak() {
        let now = 1_000_000u64;
        let created_at = now - 1000;

        let la = ledger_with_votes("aaa", 1, 0);
        let lb = ledger_with_votes("zzz", 1, 0);
        let lm = ledger_with_votes("mmm", 1, 0);

        let entries: Vec<(&str, &VoteLedger, u64)> = vec![
            ("zzz", &lb, created_at),
            ("mmm", &lm, created_at),
            ("aaa", &la, created_at),
        ];

        let result = rank_posts(&entries, now);
        assert_eq!(
            result,
            vec!["aaa", "mmm", "zzz"],
            "stable tiebreak must be ascending post_id"
        );
    }

    #[test]
    fn rank_posts_excludes_empty_post_id() {
        let now = 1_000_000u64;
        let l = ledger_with_votes("p", 1, 0);
        let empty_l = ledger_with_votes("p", 1, 0);

        let entries: Vec<(&str, &VoteLedger, u64)> = vec![
            ("", &empty_l, now - 100),
            ("  ", &empty_l, now - 100),
            ("valid", &l, now - 100),
        ];

        let result = rank_posts(&entries, now);
        assert_eq!(
            result,
            vec!["valid"],
            "empty/blank post_id must be excluded"
        );
    }

    #[test]
    fn rank_posts_higher_hot_score_wins() {
        let now = 1_000_000u64;

        let fresh_upvoted = ledger_with_votes("fresh", 10, 0);
        let old_downvoted = ledger_with_votes("old", 0, 5);

        let entries: Vec<(&str, &VoteLedger, u64)> = vec![
            ("old", &old_downvoted, now - 80_000),
            ("fresh", &fresh_upvoted, now - 100),
        ];

        let result = rank_posts(&entries, now);
        assert_eq!(result[0], "fresh", "fresh upvoted post must rank first");
    }

    #[test]
    fn controversy_score_no_panic_on_large_counts() {
        let large: u64 = u64::MAX / 2;
        let result = large.saturating_mul(u64::MAX); // must not panic
        assert!(result > 0, "saturating_mul of large values must not panic");

        let now = 1_000_000u64;
        let l = ledger_with_votes("p", 50, 50);
        let _ = l.controversy_score(); // must not panic
        let _ = l.hot_score(now - 1000, now); // must not panic
    }

    #[test]
    fn rank_posts_deterministic_repeated_calls() {
        let now = 1_000_000u64;

        let la = ledger_with_votes("alpha", 5, 1);
        let lb = ledger_with_votes("beta", 3, 2);
        let lc = ledger_with_votes("gamma", 1, 0);

        let entries: Vec<(&str, &VoteLedger, u64)> = vec![
            ("alpha", &la, now - 5000),
            ("beta", &lb, now - 1000),
            ("gamma", &lc, now - 100),
        ];

        let first = rank_posts(&entries, now);
        let second = rank_posts(&entries, now);
        assert_eq!(
            first, second,
            "repeated ranking of identical input must be identical"
        );
    }
}
