use crate::{AuthorizedSocialContext, SocialUsecaseError};

/// Recency weight constant: one day in seconds.
/// A post created exactly `now` contributes the full RECENCY_WEIGHT to its score.
/// A post older than one day contributes 0 from recency.
pub const RECENCY_WEIGHT: u64 = 86_400;

/// Cap on the engagement signal contribution.
/// Prevents extremely viral posts from completely dominating the recency signal.
pub const ENGAGEMENT_CAP: u64 = 10_000;

/// Input to the feed-ranking usecase.
///
/// `scope_ref` must match `ctx.scope_ref` for a post to appear in the ranked output;
/// [`rank_feed`] enforces cross-scope isolation directly so callers do not need to
/// pre-filter. Posts with an empty `post_id` are also excluded.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FeedRankInput {
    /// Opaque post identifier; used as stable tiebreak key (lexicographic ascending).
    pub post_id: String,
    /// Scope this post belongs to (e.g. `"person:alice"` or `"tenant:acme"`).
    /// [`rank_feed`] drops posts whose `scope_ref` differs from `ctx.scope_ref`.
    pub scope_ref: String,
    /// Creation timestamp as Unix epoch seconds. Mirrors the `now: u64` time model
    /// used by `story_purge` / `plan_story_purge` in the same crate.
    pub created_at: u64,
    /// Bounded engagement signal (e.g. likes + comments + shares aggregated by caller).
    /// Values above [`ENGAGEMENT_CAP`] are treated as [`ENGAGEMENT_CAP`].
    pub engagement_count: u64,
}

/// Returns the deterministic integer score for a single post at a given wall-clock instant.
pub fn score(input: &FeedRankInput, now: u64) -> u64 {
    let age_secs = now.saturating_sub(input.created_at);
    let recency_component = RECENCY_WEIGHT.saturating_sub(age_secs.min(RECENCY_WEIGHT));
    let engagement_component = input.engagement_count.min(ENGAGEMENT_CAP);
    recency_component + engagement_component
}

/// Rank a slice of feed posts by descending score, with stable tiebreak on `post_id`.
///
/// # Context guard
/// `ctx.validate()` is called first. Any validation error is surfaced as
/// `Err(SocialUsecaseError::Api(...))`.
///
/// # Ordering
/// Primary: descending `score(post, now)`.
/// Tiebreak: ascending `post_id` (lexicographic) — deterministic across repeated calls.
///
/// # Empty input
/// An empty slice returns `Ok(vec![])`.
pub fn rank_feed(
    ctx: &AuthorizedSocialContext,
    posts: &[FeedRankInput],
    now: u64,
) -> Result<Vec<FeedRankInput>, SocialUsecaseError> {
    ctx.validate().map_err(SocialUsecaseError::Api)?;

    let mut ranked: Vec<FeedRankInput> = posts
        .iter()
        .filter(|p| !p.post_id.is_empty() && p.scope_ref == ctx.scope_ref)
        .cloned()
        .collect();

    // Sort: descending score, then ascending post_id for stable tiebreak.
    ranked.sort_by(|a, b| {
        let sa = score(a, now);
        let sb = score(b, now);
        sb.cmp(&sa).then_with(|| a.post_id.cmp(&b.post_id))
    });

    Ok(ranked)
}

#[cfg(test)]
mod tests {
    use super::*;
    use community_social_post_composition_api::{AuthorizedSocialContext, SocialApiContext};

    fn personal_ctx() -> AuthorizedSocialContext {
        AuthorizedSocialContext {
            context: SocialApiContext::Personal,
            scope_ref: "person:u".into(),
            principal_ref: "user:u".into(),
            idempotency_key: "idem".into(),
            policy_decision_ref: "cedar:allow:feed-rank".into(),
            audit_correlation_id: "audit".into(),
        }
    }

    fn make_post(post_id: &str, created_at: u64, engagement_count: u64) -> FeedRankInput {
        FeedRankInput {
            post_id: post_id.into(),
            scope_ref: "person:u".into(),
            created_at,
            engagement_count,
        }
    }

    // ── ST1: score monotonicity ───────────────────────────────────────────────

    #[test]
    fn score_monotonic_in_engagement_at_fixed_time() {
        let now = 1_000_000u64;
        let created_at = now - 3600; // 1 hour old

        let low = make_post("p", created_at, 100);
        let mid = make_post("p", created_at, 500);
        let high = make_post("p", created_at, 5000);
        let capped = make_post("p", created_at, ENGAGEMENT_CAP + 9999);

        assert!(score(&low, now) <= score(&mid, now));
        assert!(score(&mid, now) <= score(&high, now));
        // Capped engagement should equal ENGAGEMENT_CAP contribution
        assert_eq!(
            score(&capped, now),
            score(&make_post("p", created_at, ENGAGEMENT_CAP), now)
        );
    }

    #[test]
    fn score_monotonic_decreasing_in_age_at_fixed_engagement() {
        let now = 1_000_000u64;
        let engagement = 200;

        let fresh = make_post("p", now - 60, engagement); // 1 minute old
        let hour_old = make_post("p", now - 3600, engagement); // 1 hour old
        let day_old = make_post("p", now - 86_400, engagement); // exactly 1 day old
        let ancient = make_post("p", now - 200_000, engagement); // older than RECENCY_WEIGHT

        assert!(score(&fresh, now) >= score(&hour_old, now));
        assert!(score(&hour_old, now) >= score(&day_old, now));
        assert!(score(&day_old, now) >= score(&ancient, now));
        // Ancient post contributes 0 recency; score = engagement only
        assert_eq!(score(&ancient, now), engagement);
    }

    // ── ST2: rank_feed ordering and filtering ─────────────────────────────────

    #[test]
    fn rank_feed_descending_score_ordering() {
        let now = 1_000_000u64;
        let ctx = personal_ctx();
        let posts = vec![
            make_post("old-low", now - 50_000, 10),    // low score
            make_post("fresh-high", now - 100, 9_000), // high score
            make_post("mid", now - 10_000, 500),       // mid score
        ];
        let result = rank_feed(&ctx, &posts, now).unwrap();
        assert_eq!(result.len(), 3);
        // Verify strictly descending score
        let scores: Vec<u64> = result.iter().map(|p| score(p, now)).collect();
        for w in scores.windows(2) {
            assert!(w[0] >= w[1], "expected descending scores, got {:?}", scores);
        }
        assert_eq!(result[0].post_id, "fresh-high");
    }

    #[test]
    fn rank_feed_stable_tiebreak_on_equal_score() {
        let now = 1_000_000u64;
        let ctx = personal_ctx();
        // Identical created_at and engagement_count → identical score
        let posts = vec![
            make_post("zzz", now - 1000, 100),
            make_post("aaa", now - 1000, 100),
            make_post("mmm", now - 1000, 100),
        ];
        let result = rank_feed(&ctx, &posts, now).unwrap();
        assert_eq!(result.len(), 3);
        // Tiebreak: ascending post_id
        assert_eq!(result[0].post_id, "aaa");
        assert_eq!(result[1].post_id, "mmm");
        assert_eq!(result[2].post_id, "zzz");
    }

    #[test]
    fn rank_feed_excludes_empty_post_id() {
        let now = 1_000_000u64;
        let ctx = personal_ctx();
        let posts = vec![
            make_post("", now - 100, 50), // empty post_id — excluded
            make_post("valid", now - 100, 50),
        ];
        let result = rank_feed(&ctx, &posts, now).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].post_id, "valid");
    }

    // ── ST3: edge cases ───────────────────────────────────────────────────────

    #[test]
    fn rank_feed_empty_returns_empty() {
        let ctx = personal_ctx();
        let result = rank_feed(&ctx, &[], 1_000_000).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn rank_feed_single_post_returns_single() {
        let now = 1_000_000u64;
        let ctx = personal_ctx();
        let posts = vec![make_post("solo", now - 300, 42)];
        let result = rank_feed(&ctx, &posts, now).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].post_id, "solo");
    }

    #[test]
    fn rank_feed_identical_input_same_ordering() {
        let now = 1_000_000u64;
        let ctx = personal_ctx();
        let posts = vec![
            make_post("c", now - 5000, 300),
            make_post("a", now - 1000, 100),
            make_post("b", now - 3000, 300),
        ];
        let first = rank_feed(&ctx, &posts, now).unwrap();
        let second = rank_feed(&ctx, &posts, now).unwrap();
        assert_eq!(
            first, second,
            "repeated ranking of identical input must be byte-identical"
        );
    }

    #[test]
    fn score_is_bounded_by_recency_weight_plus_engagement_cap() {
        let now = 1_000_000u64;
        let max = RECENCY_WEIGHT + ENGAGEMENT_CAP;
        assert_eq!(max, 96_400);
        for created_at in [0, now - 200_000, now - 86_400, now - 1, now, now + 5_000] {
            for engagement in [0, 1, ENGAGEMENT_CAP, ENGAGEMENT_CAP + 1, u64::MAX] {
                let observed = score(&make_post("p", created_at, engagement), now);
                assert!(observed <= max, "score {observed} exceeded bound {max}");
            }
        }
        assert_eq!(score(&make_post("p", now, u64::MAX), now), max);
        assert_eq!(score(&make_post("p", 0, 0), now), 0);
    }
}
