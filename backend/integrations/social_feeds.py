"""
Social Feed Monitor - Twitter & Google Reviews Pipeline

Monitors social media feeds and reviews for a target company, stores them
in JSON files accessible to the agent, and generates tiered market analysis
reports via heartbeat schedulers:

  - **Hourly**:  Quick pulse check — new mentions, sentiment snapshot, alerts
  - **Daily**:   Full analysis — themes, engagement stats, action items
  - **Weekly**:  Strategic digest — trend comparisons, week-over-week changes,
                 executive summary with strategic recommendations

This demonstrates a practical data pipeline that a small business owner
can understand: watch what people say about you online, store it, and
get regular summaries of market sentiment at the cadence that suits you.

For the hackathon demo, we use realistic sample data for Accelerate Okanagan.
In production, this would connect to the Twitter/X API and Google Places API.
"""

import asyncio
import json
import logging
import random
from dataclasses import dataclass, field, asdict
from datetime import datetime, timedelta
from pathlib import Path
from typing import Any, Optional

logger = logging.getLogger("tojo.social_feeds")

# ---------------------------------------------------------------------------
# Storage paths
# ---------------------------------------------------------------------------
FEEDS_DIR = Path.home() / ".tojo" / "social_feeds"
TWITTER_FILE = FEEDS_DIR / "twitter_feed.json"
REVIEWS_FILE = FEEDS_DIR / "google_reviews.json"
REPORTS_DIR = FEEDS_DIR / "reports"


# ---------------------------------------------------------------------------
# Data models
# ---------------------------------------------------------------------------

@dataclass
class Tweet:
    """A single tweet/post about the target company."""
    id: str
    author: str
    handle: str
    content: str
    timestamp: str
    likes: int = 0
    retweets: int = 0
    replies: int = 0
    sentiment: str = "neutral"  # positive | neutral | negative

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass
class GoogleReview:
    """A single Google review for the target company."""
    id: str
    author: str
    rating: int  # 1-5 stars
    text: str
    timestamp: str
    helpful_count: int = 0
    sentiment: str = "neutral"

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass
class FeedSnapshot:
    """A point-in-time snapshot of all feeds for a company."""
    company: str
    fetched_at: str
    tweets: list[Tweet] = field(default_factory=list)
    reviews: list[GoogleReview] = field(default_factory=list)
    tweet_count: int = 0
    review_count: int = 0
    avg_review_rating: float = 0.0
    sentiment_summary: dict[str, int] = field(default_factory=dict)

    def to_dict(self) -> dict[str, Any]:
        return {
            "company": self.company,
            "fetched_at": self.fetched_at,
            "tweets": [t.to_dict() for t in self.tweets],
            "reviews": [r.to_dict() for r in self.reviews],
            "tweet_count": self.tweet_count,
            "review_count": self.review_count,
            "avg_review_rating": self.avg_review_rating,
            "sentiment_summary": self.sentiment_summary,
        }


# Valid report cadences
REPORT_CADENCES = ("hourly", "daily", "weekly")

# Heartbeat intervals in seconds for each cadence
CADENCE_INTERVALS = {
    "hourly": 3600,
    "daily": 86400,
    "weekly": 604800,
}


@dataclass
class MarketReport:
    """
    Market analysis report at a specific cadence.

    The cadence controls the depth of analysis:
      - hourly:  sentiment snapshot, new mention count, alerts
      - daily:   full analysis with themes, engagement, action items
      - weekly:  strategic digest with trend comparisons and recommendations
    """
    company: str
    cadence: str  # hourly | daily | weekly
    report_date: str
    generated_at: str
    period_label: str = ""  # e.g. "3:00 PM", "March 8", "Week of March 3-9"
    tweet_summary: dict[str, Any] = field(default_factory=dict)
    review_summary: dict[str, Any] = field(default_factory=dict)
    sentiment_trend: str = ""
    key_themes: list[str] = field(default_factory=list)
    action_items: list[str] = field(default_factory=list)
    market_pulse: str = ""
    # Weekly-only fields
    weekly_comparison: dict[str, Any] = field(default_factory=dict)
    strategic_recommendations: list[str] = field(default_factory=list)

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


# Backwards-compatible alias
DailyReport = MarketReport


# ---------------------------------------------------------------------------
# Demo data for Accelerate Okanagan
# ---------------------------------------------------------------------------

def _generate_demo_tweets() -> list[Tweet]:
    """Generate realistic demo tweets about Accelerate Okanagan."""
    now = datetime.utcnow()
    tweets = [
        Tweet(
            id="tw_001",
            author="Sarah Chen",
            handle="@sarahchentech",
            content="Just finished the @AccelerateOK startup bootcamp - the mentorship sessions were incredible. Connected with 3 potential investors in one afternoon! #OkanaganTech #StartupLife",
            timestamp=(now - timedelta(hours=2)).isoformat(),
            likes=45, retweets=12, replies=8,
            sentiment="positive",
        ),
        Tweet(
            id="tw_002",
            author="Mike Rodriguez",
            handle="@mikerodriguezBC",
            content="Accelerate Okanagan's new coworking space is a game changer for remote founders. Finally a proper tech hub in Kelowna. @AccelerateOK",
            timestamp=(now - timedelta(hours=5)).isoformat(),
            likes=67, retweets=23, replies=15,
            sentiment="positive",
        ),
        Tweet(
            id="tw_003",
            author="TechBC Weekly",
            handle="@techbcweekly",
            content="Report: Okanagan tech sector grew 18% in 2025, with @AccelerateOK-supported startups accounting for $42M in new revenue. Full analysis: [link]",
            timestamp=(now - timedelta(hours=8)).isoformat(),
            likes=134, retweets=56, replies=22,
            sentiment="positive",
        ),
        Tweet(
            id="tw_004",
            author="Jordan Park",
            handle="@jordanpark_dev",
            content="Applied to the @AccelerateOK Venture program 3 months ago, still waiting to hear back. The application process could use some improvement tbh.",
            timestamp=(now - timedelta(hours=12)).isoformat(),
            likes=8, retweets=1, replies=3,
            sentiment="negative",
        ),
        Tweet(
            id="tw_005",
            author="Kelowna Chamber",
            handle="@KelownaChamber",
            content="Proud to partner with @AccelerateOK on the 2026 Innovation Awards. Nominations are open for tech companies making an impact in the Okanagan region.",
            timestamp=(now - timedelta(hours=18)).isoformat(),
            likes=89, retweets=34, replies=11,
            sentiment="positive",
        ),
        Tweet(
            id="tw_006",
            author="Emily Tran",
            handle="@emilytran_ux",
            content="The @AccelerateOK pitch night was fine but I wish they had more diversity in the judging panel. Only 1 woman out of 5 judges? We can do better. #DiversityInTech",
            timestamp=(now - timedelta(hours=24)).isoformat(),
            likes=52, retweets=18, replies=27,
            sentiment="negative",
        ),
        Tweet(
            id="tw_007",
            author="DataFlow Systems",
            handle="@dataflowsys",
            content="Excited to announce we just graduated from @AccelerateOK's acceleration program! Raised our seed round and hiring 5 engineers. Thank you to the whole AO team!",
            timestamp=(now - timedelta(hours=30)).isoformat(),
            likes=201, retweets=78, replies=45,
            sentiment="positive",
        ),
        Tweet(
            id="tw_008",
            author="Ben Okafor",
            handle="@benokafor",
            content="Had coffee with the @AccelerateOK team yesterday. Their knowledge of the BC tech ecosystem is unmatched. Good advice on market entry strategy for our SaaS.",
            timestamp=(now - timedelta(hours=36)).isoformat(),
            likes=34, retweets=6, replies=4,
            sentiment="positive",
        ),
        Tweet(
            id="tw_009",
            author="Startup Grind Kelowna",
            handle="@SGKelowna",
            content="Tonight's fireside chat with @AccelerateOK's CEO was standing room only. Key takeaway: the Okanagan is becoming Canada's next major tech corridor. #StartupGrind",
            timestamp=(now - timedelta(hours=42)).isoformat(),
            likes=76, retweets=29, replies=13,
            sentiment="positive",
        ),
        Tweet(
            id="tw_010",
            author="Alex Kim",
            handle="@alexkim_startup",
            content="Is @AccelerateOK still relevant in 2026? With so many online accelerators, I'm wondering if a regional focus is a strength or a limitation. Thoughts? #BCTech",
            timestamp=(now - timedelta(hours=48)).isoformat(),
            likes=23, retweets=5, replies=19,
            sentiment="neutral",
        ),
        Tweet(
            id="tw_011",
            author="Okanagan Innovates",
            handle="@OKInnovates",
            content="Great workshop at @AccelerateOK today on AI adoption for small businesses. The hands-on demos made complex concepts accessible to everyone in the room.",
            timestamp=(now - timedelta(hours=52)).isoformat(),
            likes=41, retweets=14, replies=6,
            sentiment="positive",
        ),
        Tweet(
            id="tw_012",
            author="Priya Sharma",
            handle="@priyasharma_pm",
            content="@AccelerateOK's mentorship matching could be better. Got paired with someone outside my industry. Feedback submitted, hoping they iterate on this.",
            timestamp=(now - timedelta(hours=60)).isoformat(),
            likes=11, retweets=2, replies=7,
            sentiment="negative",
        ),
    ]
    return tweets


def _generate_demo_reviews() -> list[GoogleReview]:
    """Generate realistic Google reviews for Accelerate Okanagan."""
    now = datetime.utcnow()
    reviews = [
        GoogleReview(
            id="gr_001",
            author="Marcus W.",
            rating=5,
            text="Accelerate Okanagan completely changed the trajectory of our startup. The mentors are experienced, the network is invaluable, and the programs are well-structured. Can't recommend enough for any tech founder in BC.",
            timestamp=(now - timedelta(days=3)).isoformat(),
            helpful_count=14,
            sentiment="positive",
        ),
        GoogleReview(
            id="gr_002",
            author="Lisa Park",
            rating=4,
            text="Good resources and networking events. The coworking space is modern and well-maintained. Only giving 4 stars because some of the programming feels geared toward later-stage companies. More support for idea-stage founders would be great.",
            timestamp=(now - timedelta(days=7)).isoformat(),
            helpful_count=9,
            sentiment="positive",
        ),
        GoogleReview(
            id="gr_003",
            author="David Chen",
            rating=5,
            text="The venture acceleration program was exactly what we needed. Helped us refine our pitch, connected us with angel investors, and their demo day had real decision-makers in the audience. Raised $500K within 2 months of graduating.",
            timestamp=(now - timedelta(days=12)).isoformat(),
            helpful_count=22,
            sentiment="positive",
        ),
        GoogleReview(
            id="gr_004",
            author="Amara Johnson",
            rating=3,
            text="Mixed experience. The events and workshops are excellent, but the actual acceleration program felt a bit generic. I was hoping for more tailored advice for our specific market (agtech). Still, the community alone makes it worthwhile.",
            timestamp=(now - timedelta(days=18)).isoformat(),
            helpful_count=7,
            sentiment="neutral",
        ),
        GoogleReview(
            id="gr_005",
            author="Ryan Foster",
            rating=5,
            text="As a solo founder, AO gave me the support system I desperately needed. Weekly check-ins with my mentor, access to legal and accounting resources, and a community of founders who actually understand the grind. 10/10.",
            timestamp=(now - timedelta(days=22)).isoformat(),
            helpful_count=18,
            sentiment="positive",
        ),
        GoogleReview(
            id="gr_006",
            author="Nina Kowalski",
            rating=2,
            text="Applied twice and was rejected both times without meaningful feedback. The application process feels like a black box. Would appreciate more transparency about what they're looking for in startups.",
            timestamp=(now - timedelta(days=28)).isoformat(),
            helpful_count=11,
            sentiment="negative",
        ),
        GoogleReview(
            id="gr_007",
            author="James Liu",
            rating=4,
            text="Great organization doing important work for the Okanagan tech ecosystem. Their events bring together founders, investors, and corporate partners effectively. The new downtown space is impressive.",
            timestamp=(now - timedelta(days=35)).isoformat(),
            helpful_count=6,
            sentiment="positive",
        ),
        GoogleReview(
            id="gr_008",
            author="Tanya Redbird",
            rating=5,
            text="Accelerate Okanagan helped our Indigenous-led tech company navigate the startup world. The team was respectful, knowledgeable, and genuinely invested in our success. They're building something special in Kelowna.",
            timestamp=(now - timedelta(days=40)).isoformat(),
            helpful_count=31,
            sentiment="positive",
        ),
        GoogleReview(
            id="gr_009",
            author="Carlos Mendez",
            rating=4,
            text="Solid accelerator program with good connections in the BC tech scene. The workshops on product-market fit and go-to-market strategy were particularly useful. Would love to see more focus on international expansion.",
            timestamp=(now - timedelta(days=45)).isoformat(),
            helpful_count=5,
            sentiment="positive",
        ),
        GoogleReview(
            id="gr_010",
            author="Hannah Wright",
            rating=3,
            text="The networking events are the real value here. Met my co-founder at an AO mixer. The formal programs are decent but not groundbreaking compared to what you can find online. Location is convenient though.",
            timestamp=(now - timedelta(days=50)).isoformat(),
            helpful_count=8,
            sentiment="neutral",
        ),
    ]
    return reviews


# ---------------------------------------------------------------------------
# Sentiment analysis (rule-based for demo; LLM would handle this in prod)
# ---------------------------------------------------------------------------

POSITIVE_WORDS = {
    "great", "excellent", "amazing", "incredible", "love", "best", "fantastic",
    "wonderful", "perfect", "outstanding", "recommended", "impressed", "valuable",
    "helpful", "game changer", "excited", "proud", "invaluable", "success",
    "changed", "special", "invested", "raised", "grew", "hiring",
}

NEGATIVE_WORDS = {
    "bad", "terrible", "awful", "poor", "worst", "disappointed", "frustrated",
    "waste", "rejected", "black box", "limitation", "generic", "waiting",
    "improvement", "better", "could be", "wish", "unfortunately",
}


def _classify_sentiment(text: str) -> str:
    """Simple keyword-based sentiment classification."""
    lower = text.lower()
    pos = sum(1 for w in POSITIVE_WORDS if w in lower)
    neg = sum(1 for w in NEGATIVE_WORDS if w in lower)
    if pos > neg:
        return "positive"
    elif neg > pos:
        return "negative"
    return "neutral"


# ---------------------------------------------------------------------------
# Social Feed Monitor
# ---------------------------------------------------------------------------

class SocialFeedMonitor:
    """
    Monitors Twitter and Google Reviews for a target company.

    Stores feed data in JSON files that are accessible to the agent for
    analysis. Provides heartbeat schedulers that generate reports at three
    cadences: hourly (quick pulse), daily (full analysis), and weekly
    (strategic digest).

    Usage:
        monitor = SocialFeedMonitor("Accelerate Okanagan")
        snapshot = await monitor.fetch_feeds()
        report = monitor.generate_report("daily")
        monitor.start_heartbeat("daily")   # one cadence
        monitor.start_heartbeat("hourly")  # can run multiple
    """

    def __init__(self, company: str = "Accelerate Okanagan"):
        self.company = company
        # One task per cadence so they can run independently
        self._heartbeat_tasks: dict[str, asyncio.Task] = {}
        self._heartbeat_running: dict[str, bool] = {
            "hourly": False,
            "daily": False,
            "weekly": False,
        }
        self._ensure_storage()

    def _ensure_storage(self) -> None:
        """Create storage directories if they don't exist."""
        FEEDS_DIR.mkdir(parents=True, exist_ok=True)
        REPORTS_DIR.mkdir(parents=True, exist_ok=True)

    # -- Feed fetching -------------------------------------------------------

    async def fetch_feeds(self) -> FeedSnapshot:
        """
        Fetch latest tweets and reviews for the target company.

        In production, this calls the Twitter API and Google Places API.
        For the demo, it returns realistic sample data.
        """
        logger.info("Fetching social feeds for %s", self.company)

        tweets = _generate_demo_tweets()
        reviews = _generate_demo_reviews()

        # Build snapshot
        all_sentiments = (
            [t.sentiment for t in tweets] + [r.sentiment for r in reviews]
        )
        sentiment_counts = {
            "positive": sum(1 for s in all_sentiments if s == "positive"),
            "negative": sum(1 for s in all_sentiments if s == "negative"),
            "neutral": sum(1 for s in all_sentiments if s == "neutral"),
        }

        avg_rating = (
            sum(r.rating for r in reviews) / len(reviews) if reviews else 0.0
        )

        snapshot = FeedSnapshot(
            company=self.company,
            fetched_at=datetime.utcnow().isoformat(),
            tweets=tweets,
            reviews=reviews,
            tweet_count=len(tweets),
            review_count=len(reviews),
            avg_review_rating=round(avg_rating, 2),
            sentiment_summary=sentiment_counts,
        )

        # Persist to JSON
        self._save_feeds(snapshot)
        logger.info(
            "Fetched %d tweets and %d reviews for %s",
            len(tweets), len(reviews), self.company,
        )
        return snapshot

    def _save_feeds(self, snapshot: FeedSnapshot) -> None:
        """Save feed data to JSON files."""
        TWITTER_FILE.write_text(json.dumps(
            {"company": self.company, "tweets": [t.to_dict() for t in snapshot.tweets]},
            indent=2,
        ))
        REVIEWS_FILE.write_text(json.dumps(
            {"company": self.company, "reviews": [r.to_dict() for r in snapshot.reviews]},
            indent=2,
        ))
        logger.info("Feed data saved to %s", FEEDS_DIR)

    def load_feeds(self) -> FeedSnapshot:
        """Load the most recently saved feed data from disk."""
        tweets = []
        reviews = []

        if TWITTER_FILE.exists():
            data = json.loads(TWITTER_FILE.read_text())
            tweets = [Tweet(**t) for t in data.get("tweets", [])]

        if REVIEWS_FILE.exists():
            data = json.loads(REVIEWS_FILE.read_text())
            reviews = [GoogleReview(**r) for r in data.get("reviews", [])]

        all_sentiments = (
            [t.sentiment for t in tweets] + [r.sentiment for r in reviews]
        )
        sentiment_counts = {
            "positive": sum(1 for s in all_sentiments if s == "positive"),
            "negative": sum(1 for s in all_sentiments if s == "negative"),
            "neutral": sum(1 for s in all_sentiments if s == "neutral"),
        }
        avg_rating = (
            sum(r.rating for r in reviews) / len(reviews) if reviews else 0.0
        )

        return FeedSnapshot(
            company=self.company,
            fetched_at=datetime.utcnow().isoformat(),
            tweets=tweets,
            reviews=reviews,
            tweet_count=len(tweets),
            review_count=len(reviews),
            avg_review_rating=round(avg_rating, 2),
            sentiment_summary=sentiment_counts,
        )

    # -- Report generation ---------------------------------------------------

    def generate_report(
        self,
        cadence: str = "daily",
        snapshot: Optional[FeedSnapshot] = None,
    ) -> MarketReport:
        """
        Generate a market analysis report at the specified cadence.

        Args:
            cadence: "hourly", "daily", or "weekly"
            snapshot: Feed data (loaded from disk if not provided)

        Returns:
            MarketReport with depth matching the cadence
        """
        if cadence not in REPORT_CADENCES:
            raise ValueError(f"Invalid cadence '{cadence}'. Must be one of {REPORT_CADENCES}")

        if snapshot is None:
            snapshot = self.load_feeds()

        now = datetime.utcnow()

        # Core stats used by all cadences
        tweet_summary = self._build_tweet_summary(snapshot)
        review_summary = self._build_review_summary(snapshot)
        sentiment_trend = self._compute_sentiment_trend(snapshot)

        # Period label depends on cadence
        if cadence == "hourly":
            period_label = now.strftime("%I:%M %p")
        elif cadence == "daily":
            period_label = now.strftime("%B %d, %Y")
        else:
            week_start = now - timedelta(days=now.weekday())
            week_end = week_start + timedelta(days=6)
            period_label = f"Week of {week_start.strftime('%B %d')} - {week_end.strftime('%B %d, %Y')}"

        # Build report — depth scales with cadence
        report = MarketReport(
            company=self.company,
            cadence=cadence,
            report_date=now.strftime("%Y-%m-%d"),
            generated_at=now.isoformat(),
            period_label=period_label,
            tweet_summary=tweet_summary,
            review_summary=review_summary,
            sentiment_trend=sentiment_trend,
        )

        # Daily and weekly get themes and action items
        if cadence in ("daily", "weekly"):
            report.key_themes = self._extract_themes(snapshot)
            report.action_items = self._generate_action_items(snapshot, tweet_summary, review_summary)

        # All cadences get a market pulse, but depth varies
        report.market_pulse = self._generate_market_pulse(snapshot, sentiment_trend, cadence)

        # Weekly gets strategic extras
        if cadence == "weekly":
            report.weekly_comparison = self._build_weekly_comparison(snapshot)
            report.strategic_recommendations = self._generate_strategic_recommendations(snapshot)

        self._save_report(report)
        return report

    def _build_tweet_summary(self, snapshot: FeedSnapshot) -> dict[str, Any]:
        """Compute tweet statistics from a snapshot."""
        tweet_engagement = sum(
            t.likes + t.retweets + t.replies for t in snapshot.tweets
        )
        top_tweet = max(
            snapshot.tweets,
            key=lambda t: t.likes + t.retweets + t.replies,
            default=None,
        )
        sentiments = [t.sentiment for t in snapshot.tweets]
        return {
            "total_tweets": len(snapshot.tweets),
            "total_engagement": tweet_engagement,
            "avg_engagement": round(tweet_engagement / max(len(snapshot.tweets), 1), 1),
            "sentiment_breakdown": {
                "positive": sentiments.count("positive"),
                "neutral": sentiments.count("neutral"),
                "negative": sentiments.count("negative"),
            },
            "top_tweet": top_tweet.to_dict() if top_tweet else None,
        }

    def _build_review_summary(self, snapshot: FeedSnapshot) -> dict[str, Any]:
        """Compute review statistics from a snapshot."""
        ratings = [r.rating for r in snapshot.reviews]
        sentiments = [r.sentiment for r in snapshot.reviews]
        return {
            "total_reviews": len(snapshot.reviews),
            "average_rating": round(sum(ratings) / max(len(ratings), 1), 2),
            "rating_distribution": {
                str(i): ratings.count(i) for i in range(1, 6)
            },
            "sentiment_breakdown": {
                "positive": sentiments.count("positive"),
                "neutral": sentiments.count("neutral"),
                "negative": sentiments.count("negative"),
            },
        }

    def _compute_sentiment_trend(self, snapshot: FeedSnapshot) -> str:
        """Compute an overall sentiment trend string."""
        tweet_sentiments = [t.sentiment for t in snapshot.tweets]
        review_sentiments = [r.sentiment for r in snapshot.reviews]
        total_positive = (
            tweet_sentiments.count("positive") + review_sentiments.count("positive")
        )
        total_negative = (
            tweet_sentiments.count("negative") + review_sentiments.count("negative")
        )
        total = len(tweet_sentiments) + len(review_sentiments)

        if total == 0:
            return "No data available"

        pos_pct = round(total_positive / total * 100)
        neg_pct = round(total_negative / total * 100)
        if pos_pct >= 70:
            return f"Very Positive ({pos_pct}% positive mentions)"
        elif pos_pct >= 50:
            return f"Mostly Positive ({pos_pct}% positive, {neg_pct}% negative)"
        elif neg_pct >= 50:
            return f"Concerning ({neg_pct}% negative mentions - needs attention)"
        return f"Mixed ({pos_pct}% positive, {neg_pct}% negative)"

    def _extract_themes(self, snapshot: FeedSnapshot) -> list[str]:
        """Extract recurring themes from tweets and reviews."""
        all_text = " ".join(
            [t.content for t in snapshot.tweets] +
            [r.text for r in snapshot.reviews]
        ).lower()

        theme_keywords = {
            "Mentorship & Support": ["mentor", "support", "guidance", "advice", "check-in"],
            "Networking & Community": ["network", "community", "connect", "partner", "mixer"],
            "Startup Programs": ["program", "accelerat", "bootcamp", "venture", "demo day"],
            "Coworking & Facilities": ["space", "coworking", "office", "downtown", "facility"],
            "Funding & Investment": ["invest", "raised", "seed", "angel", "funding", "revenue"],
            "Diversity & Inclusion": ["diversity", "inclusion", "indigenous", "women", "underrepresented"],
            "Tech Ecosystem Growth": ["ecosystem", "grew", "growth", "corridor", "sector"],
            "Application Process": ["application", "applied", "rejected", "process", "feedback"],
        }

        themes = []
        for theme, keywords in theme_keywords.items():
            hits = sum(1 for kw in keywords if kw in all_text)
            if hits >= 2:
                themes.append(theme)

        return themes[:6]

    def _generate_action_items(
        self,
        snapshot: FeedSnapshot,
        tweet_summary: dict,
        review_summary: dict,
    ) -> list[str]:
        """Generate actionable recommendations for the business owner."""
        items = []

        neg_tweets = tweet_summary["sentiment_breakdown"]["negative"]
        if neg_tweets >= 3:
            items.append(
                f"ATTENTION: {neg_tweets} negative tweets detected. Review and consider "
                "responding to address concerns publicly."
            )

        avg_rating = review_summary["average_rating"]
        if avg_rating < 4.0:
            items.append(
                f"Average Google rating is {avg_rating}/5. Identify common complaints "
                "and develop an improvement plan."
            )

        top = tweet_summary.get("top_tweet")
        if top and (top["likes"] + top["retweets"]) > 100:
            items.append(
                "A tweet about your organization went viral (100+ engagements). "
                "Consider amplifying this content and engaging with the conversation."
            )

        neg_reviews = [r for r in snapshot.reviews if r.sentiment == "negative"]
        if neg_reviews:
            items.append(
                f"{len(neg_reviews)} negative review(s) detected. Consider reaching out "
                "to these reviewers to address their concerns and show responsiveness."
            )

        items.append(
            "Engage with positive mentions by liking, retweeting, and thanking supporters. "
            "This strengthens community relationships and boosts visibility."
        )

        return items

    def _generate_market_pulse(
        self, snapshot: FeedSnapshot, sentiment_trend: str, cadence: str = "daily",
    ) -> str:
        """Generate an executive summary whose depth matches the cadence."""
        now = datetime.utcnow()
        tweet_count = snapshot.tweet_count
        review_count = snapshot.review_count
        avg_rating = snapshot.avg_review_rating

        if cadence == "hourly":
            return (
                f"**Hourly Pulse for {self.company}** ({now.strftime('%I:%M %p, %B %d')})\n\n"
                f"Monitoring {tweet_count} tweets and {review_count} reviews. "
                f"Sentiment: **{sentiment_trend.lower()}**. "
                f"Google rating: **{avg_rating}/5**."
            )

        if cadence == "weekly":
            return (
                f"**Weekly Strategic Digest for {self.company}** "
                f"({now.strftime('%B %d, %Y')})\n\n"
                f"This week, {tweet_count} tweets and {review_count} Google reviews "
                f"were tracked across all channels. The overall sentiment is "
                f"**{sentiment_trend.lower()}**. Google Reviews average "
                f"**{avg_rating}/5 stars**.\n\n"
                f"The Okanagan tech community is actively discussing {self.company}'s "
                f"programs, with the strongest positive signals around mentorship "
                f"quality, networking events, and ecosystem growth. Areas flagged for "
                f"improvement include application transparency and diversity in "
                f"leadership panels.\n\n"
                f"**Weekly Recommendation:** Review this week's negative mentions "
                f"for recurring patterns. Consider scheduling a community Q&A to "
                f"address the most common concerns and demonstrate responsiveness."
            )

        # daily (default)
        return (
            f"**Market Pulse for {self.company}** ({now.strftime('%B %d, %Y')})\n\n"
            f"Over the monitoring period, {tweet_count} tweets and {review_count} Google "
            f"reviews were tracked. The overall sentiment is **{sentiment_trend.lower()}**. "
            f"Google Reviews average **{avg_rating}/5 stars**.\n\n"
            f"The Okanagan tech community is actively discussing {self.company}'s programs, "
            f"with the strongest positive signals around mentorship quality, networking "
            f"events, and ecosystem growth. Areas flagged for improvement include "
            f"application transparency and diversity in leadership panels.\n\n"
            f"**Recommendation:** Focus engagement efforts on amplifying positive stories "
            f"from program graduates while proactively addressing process feedback."
        )

    def _build_weekly_comparison(self, snapshot: FeedSnapshot) -> dict[str, Any]:
        """Build week-over-week comparison data for the weekly report."""
        # In production this would compare against the previous week's stored data.
        # For the demo, we generate plausible comparison metrics.
        tweet_engagement = sum(
            t.likes + t.retweets + t.replies for t in snapshot.tweets
        )
        return {
            "tweets_this_week": snapshot.tweet_count,
            "tweets_change": "+3 vs last week",
            "engagement_this_week": tweet_engagement,
            "engagement_change": "+12% vs last week",
            "reviews_this_week": snapshot.review_count,
            "reviews_change": "+1 vs last week",
            "avg_rating_this_week": snapshot.avg_review_rating,
            "avg_rating_change": "+0.1 vs last week",
            "sentiment_direction": "improving",
        }

    def _generate_strategic_recommendations(self, snapshot: FeedSnapshot) -> list[str]:
        """Generate higher-level strategic recommendations for the weekly report."""
        recs = []

        # Analyze engagement distribution
        high_engagement = [
            t for t in snapshot.tweets
            if (t.likes + t.retweets + t.replies) > 50
        ]
        if high_engagement:
            topics = ", ".join(
                t.content[:60].split(" - ")[0].split("@")[0].strip()
                for t in high_engagement[:3]
            )
            recs.append(
                f"**Content strategy:** {len(high_engagement)} tweets had above-average "
                f"engagement. High-performing topics include: {topics}. "
                f"Create more content in these areas."
            )

        # Review rating trend
        five_star = sum(1 for r in snapshot.reviews if r.rating == 5)
        total = len(snapshot.reviews)
        if total > 0 and five_star / total >= 0.5:
            recs.append(
                f"**Review strength:** {five_star}/{total} reviews are 5-star. "
                f"Encourage satisfied participants to share their reviews — social "
                f"proof is your strongest growth lever."
            )

        # Negative pattern detection
        neg_reviews = [r for r in snapshot.reviews if r.sentiment == "negative"]
        neg_tweets = [t for t in snapshot.tweets if t.sentiment == "negative"]
        if neg_reviews or neg_tweets:
            neg_texts = [r.text for r in neg_reviews] + [t.content for t in neg_tweets]
            combined = " ".join(neg_texts).lower()
            pain_points = []
            if "application" in combined or "rejected" in combined or "process" in combined:
                pain_points.append("application process transparency")
            if "diversity" in combined or "women" in combined or "panel" in combined:
                pain_points.append("diversity in leadership/judging")
            if "generic" in combined or "tailored" in combined or "specific" in combined:
                pain_points.append("program personalization")
            if pain_points:
                recs.append(
                    f"**Address recurring pain points:** Negative feedback clusters around: "
                    f"{', '.join(pain_points)}. Consider a targeted improvement initiative."
                )

        recs.append(
            f"**Community building:** Host a monthly 'Founder Spotlight' showcasing "
            f"program graduates. This generates positive social proof and strengthens "
            f"the alumni network."
        )

        return recs

    def _save_report(self, report: MarketReport) -> Path:
        """Save a report to disk, organized by cadence."""
        cadence_dir = REPORTS_DIR / report.cadence
        cadence_dir.mkdir(parents=True, exist_ok=True)
        if report.cadence == "hourly":
            timestamp = datetime.utcnow().strftime("%Y%m%d_%H%M")
            filename = f"hourly_report_{timestamp}.json"
        elif report.cadence == "weekly":
            filename = f"weekly_report_{report.report_date}.json"
        else:
            filename = f"daily_report_{report.report_date}.json"
        path = cadence_dir / filename
        path.write_text(json.dumps(report.to_dict(), indent=2))
        logger.info("%s report saved to %s", report.cadence.title(), path)
        return path

    def list_reports(self, cadence: Optional[str] = None) -> list[dict[str, str]]:
        """
        List saved reports, optionally filtered by cadence.

        Args:
            cadence: If given, only list reports of this cadence.
                     If None, list all cadences.
        """
        if not REPORTS_DIR.exists():
            return []

        cadences = [cadence] if cadence else list(REPORT_CADENCES)
        reports = []
        for c in cadences:
            cadence_dir = REPORTS_DIR / c
            if not cadence_dir.exists():
                # Also check flat directory for backwards compatibility
                continue
            for p in sorted(cadence_dir.glob("*.json"), reverse=True):
                reports.append({
                    "filename": p.name,
                    "path": str(p),
                    "cadence": c,
                    "date": p.stem.split("_report_")[-1] if "_report_" in p.stem else p.stem,
                    "modified": datetime.fromtimestamp(p.stat().st_mtime).isoformat(),
                })

        # Also check legacy flat directory for old daily_report_* files
        for p in sorted(REPORTS_DIR.glob("daily_report_*.json"), reverse=True):
            reports.append({
                "filename": p.name,
                "path": str(p),
                "cadence": "daily",
                "date": p.stem.replace("daily_report_", ""),
                "modified": datetime.fromtimestamp(p.stat().st_mtime).isoformat(),
            })

        return reports

    def load_report(self, path: str) -> dict[str, Any]:
        """Load a saved report from disk."""
        return json.loads(Path(path).read_text())

    # -- Heartbeat scheduler -------------------------------------------------

    def start_heartbeat(self, cadence: str = "daily") -> bool:
        """
        Start a heartbeat scheduler for the given cadence.

        Multiple cadences can run simultaneously (e.g. hourly + daily + weekly).

        Args:
            cadence: "hourly", "daily", or "weekly"

        Returns:
            True if started, False if already running for that cadence
        """
        if cadence not in REPORT_CADENCES:
            raise ValueError(f"Invalid cadence '{cadence}'. Must be one of {REPORT_CADENCES}")

        if self._heartbeat_running.get(cadence, False):
            logger.warning("Heartbeat already running for cadence: %s", cadence)
            return False

        interval = CADENCE_INTERVALS[cadence]
        self._heartbeat_running[cadence] = True
        self._heartbeat_tasks[cadence] = asyncio.create_task(
            self._heartbeat_loop(cadence, interval)
        )
        logger.info(
            "Heartbeat started for %s [%s] (interval: %ds)",
            self.company, cadence, interval,
        )
        return True

    def stop_heartbeat(self, cadence: Optional[str] = None) -> bool:
        """
        Stop heartbeat scheduler(s).

        Args:
            cadence: Stop a specific cadence, or None to stop all.

        Returns:
            True if any heartbeat was stopped
        """
        cadences = [cadence] if cadence else list(REPORT_CADENCES)
        stopped_any = False

        for c in cadences:
            if not self._heartbeat_running.get(c, False):
                continue
            self._heartbeat_running[c] = False
            task = self._heartbeat_tasks.pop(c, None)
            if task:
                task.cancel()
            stopped_any = True
            logger.info("Heartbeat stopped for %s [%s]", self.company, c)

        return stopped_any

    @property
    def heartbeat_active(self) -> bool:
        """True if any heartbeat cadence is running."""
        return any(self._heartbeat_running.values())

    def heartbeat_status(self) -> dict[str, bool]:
        """Return running state for each cadence."""
        return dict(self._heartbeat_running)

    async def _heartbeat_loop(self, cadence: str, interval: int) -> None:
        """Internal heartbeat loop for a single cadence."""
        try:
            while self._heartbeat_running.get(cadence, False):
                logger.info("Heartbeat tick [%s]: fetching feeds for %s", cadence, self.company)
                try:
                    snapshot = await self.fetch_feeds()
                    report = self.generate_report(cadence, snapshot)
                    logger.info(
                        "Heartbeat %s report generated: %s (sentiment: %s)",
                        cadence, report.report_date, report.sentiment_trend,
                    )
                except Exception as exc:
                    logger.error("Heartbeat [%s] fetch/report failed: %s", cadence, exc)

                await asyncio.sleep(interval)
        except asyncio.CancelledError:
            logger.info("Heartbeat loop [%s] cancelled", cadence)

    # -- Chat handler --------------------------------------------------------

    async def handle_chat(self, message: str, context: dict) -> str:
        """Handle social feed requests from the chat interface."""
        lower = message.lower()

        if "fetch" in lower or "refresh" in lower or "update" in lower:
            snapshot = await self.fetch_feeds()
            return self._format_snapshot_text(snapshot)

        # Report requests — detect cadence from message
        if "report" in lower or "analysis" in lower or "summary" in lower:
            cadence = self._detect_cadence(lower)
            snapshot = await self.fetch_feeds()
            report = self.generate_report(cadence, snapshot)
            return self._format_report_text(report)

        # Heartbeat start — detect cadence(s)
        if "heartbeat" in lower and ("start" in lower or "enable" in lower):
            cadences = self._detect_cadences_multi(lower)
            results = []
            for c in cadences:
                started = self.start_heartbeat(c)
                if started:
                    interval = CADENCE_INTERVALS[c]
                    label = "hour" if c == "hourly" else ("day" if c == "daily" else "week")
                    results.append(f"- **{c.title()}** heartbeat started (every {label})")
                else:
                    results.append(f"- **{c.title()}** heartbeat was already running")

            status = self.heartbeat_status()
            active = [c for c, running in status.items() if running]
            return (
                f"Heartbeat monitoring for **{self.company}**:\n\n"
                + "\n".join(results)
                + f"\n\nActive schedules: {', '.join(active) if active else 'none'}"
            )

        # Heartbeat stop
        if "heartbeat" in lower and ("stop" in lower or "disable" in lower):
            cadence = self._detect_cadence(lower)
            if cadence == "daily" and "all" in lower:
                stopped = self.stop_heartbeat()  # Stop all
            else:
                stopped = self.stop_heartbeat(cadence)
            if stopped:
                return f"Heartbeat monitoring stopped for **{self.company}** ({cadence})."
            return "Heartbeat is not currently running."

        # Default: show overview and options
        snapshot = await self.fetch_feeds()
        return self._format_feed_overview(snapshot)

    def _detect_cadence(self, text: str) -> str:
        """Detect a single cadence from user message text."""
        if "hourly" in text or "hour" in text:
            return "hourly"
        if "weekly" in text or "week" in text:
            return "weekly"
        return "daily"

    def _detect_cadences_multi(self, text: str) -> list[str]:
        """Detect one or more cadences from user message text."""
        if "all" in text:
            return list(REPORT_CADENCES)
        found = []
        if "hourly" in text or "hour" in text:
            found.append("hourly")
        if "daily" in text or "day" in text:
            found.append("daily")
        if "weekly" in text or "week" in text:
            found.append("weekly")
        return found if found else ["daily"]

    def _format_feed_overview(self, snapshot: FeedSnapshot) -> str:
        """Format a brief overview of the social feeds."""
        status = self.heartbeat_status()
        active = [c for c, running in status.items() if running]
        hb_text = ", ".join(active) if active else "none"

        return (
            f"## Social Feed Monitor - {self.company}\n\n"
            f"**Twitter/X:** {snapshot.tweet_count} recent tweets tracked\n"
            f"**Google Reviews:** {snapshot.review_count} reviews ({snapshot.avg_review_rating}/5 avg)\n"
            f"**Overall Sentiment:** {snapshot.sentiment_summary}\n\n"
            "**Reports available at three cadences:**\n"
            "- **\"hourly report\"** — quick pulse check with sentiment snapshot\n"
            "- **\"daily report\"** — full analysis with themes and action items\n"
            "- **\"weekly report\"** — strategic digest with trends and recommendations\n\n"
            "**Other commands:**\n"
            "- **\"refresh feeds\"** — fetch the latest data\n"
            "- **\"start heartbeat all\"** — enable all three schedules\n"
            "- **\"start hourly/daily/weekly heartbeat\"** — enable one schedule\n"
            "- **\"stop heartbeat\"** — disable monitoring\n\n"
            f"Active heartbeats: **{hb_text}**"
        )

    def _format_snapshot_text(self, snapshot: FeedSnapshot) -> str:
        """Format a feed snapshot for display in chat."""
        lines = [
            f"## Social Feeds - {self.company}",
            f"*Fetched at {snapshot.fetched_at}*\n",
            f"### Twitter/X ({snapshot.tweet_count} tweets)",
            "",
        ]
        for t in snapshot.tweets[:5]:
            lines.append(f"**{t.author}** ({t.handle}) - {t.sentiment.upper()}")
            lines.append(f"> {t.content}")
            lines.append(f"  Likes: {t.likes} | RT: {t.retweets} | Replies: {t.replies}\n")

        if snapshot.tweet_count > 5:
            lines.append(f"*...and {snapshot.tweet_count - 5} more tweets*\n")

        lines.append(f"### Google Reviews ({snapshot.review_count} reviews, avg {snapshot.avg_review_rating}/5)")
        lines.append("")
        for r in snapshot.reviews[:5]:
            stars = "★" * r.rating + "☆" * (5 - r.rating)
            lines.append(f"**{r.author}** {stars} - {r.sentiment.upper()}")
            lines.append(f"> {r.text[:200]}{'...' if len(r.text) > 200 else ''}\n")

        if snapshot.review_count > 5:
            lines.append(f"*...and {snapshot.review_count - 5} more reviews*\n")

        lines.append("---")
        lines.append(f"**Sentiment Summary:** {snapshot.sentiment_summary}")
        return "\n".join(lines)

    def _format_report_text(self, report: MarketReport) -> str:
        """Format a market report for display in chat, adapting to cadence."""
        cadence_title = {
            "hourly": "Hourly Pulse",
            "daily": "Daily Market Analysis",
            "weekly": "Weekly Strategic Digest",
        }
        title = cadence_title.get(report.cadence, "Market Report")

        lines = [
            f"## {title} - {report.company}",
            f"*{report.period_label}*\n",
            report.market_pulse,
            "",
        ]

        # All cadences get basic stats
        lines.extend([
            "### Twitter Activity",
            f"- **{report.tweet_summary['total_tweets']}** tweets tracked",
            f"- **{report.tweet_summary['total_engagement']}** total engagements",
            f"- Avg engagement per tweet: **{report.tweet_summary['avg_engagement']}**",
            f"- Sentiment: {report.tweet_summary['sentiment_breakdown']}",
            "",
            "### Google Reviews",
            f"- **{report.review_summary['total_reviews']}** reviews",
            f"- Average rating: **{report.review_summary['average_rating']}/5**",
            f"- Rating distribution: {report.review_summary['rating_distribution']}",
            f"- Sentiment: {report.review_summary['sentiment_breakdown']}",
            "",
            f"### Sentiment Trend: {report.sentiment_trend}",
            "",
        ])

        # Daily and weekly get themes + action items
        if report.key_themes:
            lines.append("### Key Themes")
            for theme in report.key_themes:
                lines.append(f"- {theme}")
            lines.append("")

        if report.action_items:
            lines.append("### Action Items")
            for i, item in enumerate(report.action_items, 1):
                lines.append(f"{i}. {item}")
            lines.append("")

        # Weekly gets comparison + strategic recs
        if report.cadence == "weekly" and report.weekly_comparison:
            comp = report.weekly_comparison
            lines.append("### Week-over-Week Comparison")
            lines.append(f"- Tweets: **{comp.get('tweets_this_week', 0)}** ({comp.get('tweets_change', 'n/a')})")
            lines.append(f"- Engagement: **{comp.get('engagement_this_week', 0)}** ({comp.get('engagement_change', 'n/a')})")
            lines.append(f"- Reviews: **{comp.get('reviews_this_week', 0)}** ({comp.get('reviews_change', 'n/a')})")
            lines.append(f"- Avg rating: **{comp.get('avg_rating_this_week', 0)}** ({comp.get('avg_rating_change', 'n/a')})")
            lines.append(f"- Sentiment direction: **{comp.get('sentiment_direction', 'n/a')}**")
            lines.append("")

        if report.strategic_recommendations:
            lines.append("### Strategic Recommendations")
            for i, rec in enumerate(report.strategic_recommendations, 1):
                lines.append(f"{i}. {rec}")

        return "\n".join(lines)
