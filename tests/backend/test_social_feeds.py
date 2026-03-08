"""Tests for the social feed monitoring module."""

import json
import pytest
from pathlib import Path
from unittest.mock import patch

from backend.integrations.social_feeds import (
    SocialFeedMonitor,
    Tweet,
    GoogleReview,
    FeedSnapshot,
    MarketReport,
    DailyReport,
    REPORT_CADENCES,
    CADENCE_INTERVALS,
    _generate_demo_tweets,
    _generate_demo_reviews,
    _classify_sentiment,
)


# Helper to patch all storage paths to a temp directory
def _patch_storage(tmp_path):
    """Return a context manager that redirects all I/O to tmp_path."""
    return (
        patch("backend.integrations.social_feeds.FEEDS_DIR", tmp_path),
        patch("backend.integrations.social_feeds.TWITTER_FILE", tmp_path / "twitter.json"),
        patch("backend.integrations.social_feeds.REVIEWS_FILE", tmp_path / "reviews.json"),
        patch("backend.integrations.social_feeds.REPORTS_DIR", tmp_path / "reports"),
    )


# ---------------------------------------------------------------------------
# Data model tests
# ---------------------------------------------------------------------------

class TestTweet:
    def test_to_dict(self):
        t = Tweet(
            id="tw_test",
            author="Test User",
            handle="@testuser",
            content="Great product!",
            timestamp="2026-03-08T12:00:00",
            likes=10,
            retweets=3,
            replies=1,
            sentiment="positive",
        )
        d = t.to_dict()
        assert d["id"] == "tw_test"
        assert d["author"] == "Test User"
        assert d["likes"] == 10
        assert d["sentiment"] == "positive"

    def test_default_values(self):
        t = Tweet(id="x", author="A", handle="@a", content="hi", timestamp="now")
        assert t.likes == 0
        assert t.sentiment == "neutral"


class TestGoogleReview:
    def test_to_dict(self):
        r = GoogleReview(
            id="gr_test",
            author="Reviewer",
            rating=5,
            text="Excellent service",
            timestamp="2026-03-08T12:00:00",
            helpful_count=7,
            sentiment="positive",
        )
        d = r.to_dict()
        assert d["rating"] == 5
        assert d["helpful_count"] == 7

    def test_default_values(self):
        r = GoogleReview(id="x", author="A", rating=3, text="ok", timestamp="now")
        assert r.helpful_count == 0
        assert r.sentiment == "neutral"


class TestFeedSnapshot:
    def test_to_dict(self):
        snapshot = FeedSnapshot(
            company="Test Co",
            fetched_at="2026-03-08T12:00:00",
            tweets=[Tweet(id="1", author="A", handle="@a", content="hi", timestamp="now")],
            reviews=[GoogleReview(id="1", author="B", rating=4, text="good", timestamp="now")],
            tweet_count=1,
            review_count=1,
            avg_review_rating=4.0,
            sentiment_summary={"positive": 1, "neutral": 1, "negative": 0},
        )
        d = snapshot.to_dict()
        assert d["company"] == "Test Co"
        assert len(d["tweets"]) == 1
        assert len(d["reviews"]) == 1
        assert d["avg_review_rating"] == 4.0
        json.dumps(d)


class TestMarketReport:
    def test_to_dict_daily(self):
        report = MarketReport(
            company="Test Co",
            cadence="daily",
            report_date="2026-03-08",
            generated_at="2026-03-08T12:00:00",
            period_label="March 08, 2026",
            sentiment_trend="Mostly Positive",
            key_themes=["Innovation", "Support"],
            action_items=["Respond to negative reviews"],
            market_pulse="Looking good.",
        )
        d = report.to_dict()
        assert d["company"] == "Test Co"
        assert d["cadence"] == "daily"
        assert d["sentiment_trend"] == "Mostly Positive"
        assert len(d["key_themes"]) == 2
        json.dumps(d)

    def test_to_dict_hourly(self):
        report = MarketReport(
            company="Test Co",
            cadence="hourly",
            report_date="2026-03-08",
            generated_at="2026-03-08T15:00:00",
            period_label="03:00 PM",
        )
        d = report.to_dict()
        assert d["cadence"] == "hourly"
        json.dumps(d)

    def test_to_dict_weekly(self):
        report = MarketReport(
            company="Test Co",
            cadence="weekly",
            report_date="2026-03-08",
            generated_at="2026-03-08T12:00:00",
            period_label="Week of March 03 - March 09, 2026",
            weekly_comparison={"tweets_change": "+5 vs last week"},
            strategic_recommendations=["Focus on community events"],
        )
        d = report.to_dict()
        assert d["cadence"] == "weekly"
        assert d["weekly_comparison"]["tweets_change"] == "+5 vs last week"
        assert len(d["strategic_recommendations"]) == 1
        json.dumps(d)

    def test_backwards_compat_alias(self):
        """DailyReport is an alias for MarketReport."""
        assert DailyReport is MarketReport


class TestConstants:
    def test_cadences(self):
        assert "hourly" in REPORT_CADENCES
        assert "daily" in REPORT_CADENCES
        assert "weekly" in REPORT_CADENCES

    def test_intervals(self):
        assert CADENCE_INTERVALS["hourly"] == 3600
        assert CADENCE_INTERVALS["daily"] == 86400
        assert CADENCE_INTERVALS["weekly"] == 604800


# ---------------------------------------------------------------------------
# Demo data generation tests
# ---------------------------------------------------------------------------

class TestDemoData:
    def test_generate_demo_tweets(self):
        tweets = _generate_demo_tweets()
        assert len(tweets) == 12
        assert all(isinstance(t, Tweet) for t in tweets)
        assert all(t.id.startswith("tw_") for t in tweets)
        sentiments = {t.sentiment for t in tweets}
        assert "positive" in sentiments
        assert "negative" in sentiments

    def test_generate_demo_reviews(self):
        reviews = _generate_demo_reviews()
        assert len(reviews) == 10
        assert all(isinstance(r, GoogleReview) for r in reviews)
        assert all(r.id.startswith("gr_") for r in reviews)
        assert all(1 <= r.rating <= 5 for r in reviews)


# ---------------------------------------------------------------------------
# Sentiment classification tests
# ---------------------------------------------------------------------------

class TestSentimentClassification:
    def test_positive(self):
        assert _classify_sentiment("This is amazing and excellent!") == "positive"

    def test_negative(self):
        assert _classify_sentiment("Terrible experience, very disappointed.") == "negative"

    def test_neutral(self):
        assert _classify_sentiment("The office is located downtown.") == "neutral"


# ---------------------------------------------------------------------------
# SocialFeedMonitor tests
# ---------------------------------------------------------------------------

class TestSocialFeedMonitor:
    def test_init(self):
        monitor = SocialFeedMonitor("Test Company")
        assert monitor.company == "Test Company"
        assert not monitor.heartbeat_active

    def test_default_company(self):
        monitor = SocialFeedMonitor()
        assert monitor.company == "Accelerate Okanagan"

    @pytest.mark.asyncio
    async def test_fetch_feeds(self, tmp_path):
        """Test that fetching feeds returns a complete snapshot."""
        monitor = SocialFeedMonitor("Test Co")
        with _patch_storage(tmp_path)[0], _patch_storage(tmp_path)[1], \
             _patch_storage(tmp_path)[2], _patch_storage(tmp_path)[3]:
            snapshot = await monitor.fetch_feeds()

        assert snapshot.company == "Test Co"
        assert snapshot.tweet_count == 12
        assert snapshot.review_count == 10
        assert snapshot.avg_review_rating > 0
        assert "positive" in snapshot.sentiment_summary
        assert "negative" in snapshot.sentiment_summary

    @pytest.mark.asyncio
    async def test_fetch_feeds_saves_to_disk(self, tmp_path):
        """Verify that feed data is persisted to JSON files."""
        twitter_file = tmp_path / "twitter.json"
        reviews_file = tmp_path / "reviews.json"

        monitor = SocialFeedMonitor("Test Co")
        with patch("backend.integrations.social_feeds.FEEDS_DIR", tmp_path), \
             patch("backend.integrations.social_feeds.TWITTER_FILE", twitter_file), \
             patch("backend.integrations.social_feeds.REVIEWS_FILE", reviews_file), \
             patch("backend.integrations.social_feeds.REPORTS_DIR", tmp_path / "reports"):
            await monitor.fetch_feeds()

        assert twitter_file.exists()
        assert reviews_file.exists()

        twitter_data = json.loads(twitter_file.read_text())
        assert "tweets" in twitter_data
        assert len(twitter_data["tweets"]) == 12

        reviews_data = json.loads(reviews_file.read_text())
        assert "reviews" in reviews_data
        assert len(reviews_data["reviews"]) == 10

    # -- Tiered report generation tests --

    @pytest.mark.asyncio
    async def test_generate_hourly_report(self, tmp_path):
        """Hourly report: quick pulse, no themes or action items."""
        monitor = SocialFeedMonitor("Test Co")
        with patch("backend.integrations.social_feeds.FEEDS_DIR", tmp_path), \
             patch("backend.integrations.social_feeds.TWITTER_FILE", tmp_path / "twitter.json"), \
             patch("backend.integrations.social_feeds.REVIEWS_FILE", tmp_path / "reviews.json"), \
             patch("backend.integrations.social_feeds.REPORTS_DIR", tmp_path / "reports"):
            snapshot = await monitor.fetch_feeds()
            report = monitor.generate_report("hourly", snapshot)

        assert report.cadence == "hourly"
        assert report.company == "Test Co"
        assert report.sentiment_trend
        assert report.tweet_summary["total_tweets"] == 12
        assert report.review_summary["total_reviews"] == 10
        # Hourly does NOT include themes or action items
        assert report.key_themes == []
        assert report.action_items == []
        # But does include a market pulse
        assert "Hourly Pulse" in report.market_pulse
        # No weekly fields
        assert report.weekly_comparison == {}
        assert report.strategic_recommendations == []

    @pytest.mark.asyncio
    async def test_generate_daily_report(self, tmp_path):
        """Daily report: full analysis with themes and action items."""
        monitor = SocialFeedMonitor("Test Co")
        with patch("backend.integrations.social_feeds.FEEDS_DIR", tmp_path), \
             patch("backend.integrations.social_feeds.TWITTER_FILE", tmp_path / "twitter.json"), \
             patch("backend.integrations.social_feeds.REVIEWS_FILE", tmp_path / "reviews.json"), \
             patch("backend.integrations.social_feeds.REPORTS_DIR", tmp_path / "reports"):
            snapshot = await monitor.fetch_feeds()
            report = monitor.generate_report("daily", snapshot)

        assert report.cadence == "daily"
        assert len(report.key_themes) > 0
        assert len(report.action_items) > 0
        assert "Market Pulse" in report.market_pulse
        # No weekly fields
        assert report.weekly_comparison == {}
        assert report.strategic_recommendations == []

    @pytest.mark.asyncio
    async def test_generate_weekly_report(self, tmp_path):
        """Weekly report: includes comparison and strategic recommendations."""
        monitor = SocialFeedMonitor("Test Co")
        with patch("backend.integrations.social_feeds.FEEDS_DIR", tmp_path), \
             patch("backend.integrations.social_feeds.TWITTER_FILE", tmp_path / "twitter.json"), \
             patch("backend.integrations.social_feeds.REVIEWS_FILE", tmp_path / "reviews.json"), \
             patch("backend.integrations.social_feeds.REPORTS_DIR", tmp_path / "reports"):
            snapshot = await monitor.fetch_feeds()
            report = monitor.generate_report("weekly", snapshot)

        assert report.cadence == "weekly"
        assert "Week of" in report.period_label
        assert len(report.key_themes) > 0
        assert len(report.action_items) > 0
        assert "Weekly Strategic Digest" in report.market_pulse
        # Weekly extras
        assert report.weekly_comparison != {}
        assert "tweets_this_week" in report.weekly_comparison
        assert "sentiment_direction" in report.weekly_comparison
        assert len(report.strategic_recommendations) > 0

    def test_generate_report_invalid_cadence(self):
        """Invalid cadence raises ValueError."""
        monitor = SocialFeedMonitor()
        with pytest.raises(ValueError, match="Invalid cadence"):
            monitor.generate_report("biweekly")

    @pytest.mark.asyncio
    async def test_report_saved_to_cadence_directory(self, tmp_path):
        """Reports are saved in cadence-specific subdirectories."""
        reports_dir = tmp_path / "reports"

        monitor = SocialFeedMonitor("Test Co")
        with patch("backend.integrations.social_feeds.FEEDS_DIR", tmp_path), \
             patch("backend.integrations.social_feeds.TWITTER_FILE", tmp_path / "twitter.json"), \
             patch("backend.integrations.social_feeds.REVIEWS_FILE", tmp_path / "reviews.json"), \
             patch("backend.integrations.social_feeds.REPORTS_DIR", reports_dir):
            snapshot = await monitor.fetch_feeds()

            monitor.generate_report("hourly", snapshot)
            monitor.generate_report("daily", snapshot)
            monitor.generate_report("weekly", snapshot)

        # Each cadence gets its own subdirectory
        assert (reports_dir / "hourly").exists()
        assert (reports_dir / "daily").exists()
        assert (reports_dir / "weekly").exists()

        hourly_files = list((reports_dir / "hourly").glob("hourly_report_*.json"))
        daily_files = list((reports_dir / "daily").glob("daily_report_*.json"))
        weekly_files = list((reports_dir / "weekly").glob("weekly_report_*.json"))
        assert len(hourly_files) == 1
        assert len(daily_files) == 1
        assert len(weekly_files) == 1

        # Verify saved content
        saved = json.loads(daily_files[0].read_text())
        assert saved["cadence"] == "daily"
        assert saved["company"] == "Test Co"

    @pytest.mark.asyncio
    async def test_list_reports_by_cadence(self, tmp_path):
        """list_reports() can filter by cadence."""
        reports_dir = tmp_path / "reports"

        monitor = SocialFeedMonitor("Test Co")
        with patch("backend.integrations.social_feeds.FEEDS_DIR", tmp_path), \
             patch("backend.integrations.social_feeds.TWITTER_FILE", tmp_path / "twitter.json"), \
             patch("backend.integrations.social_feeds.REVIEWS_FILE", tmp_path / "reviews.json"), \
             patch("backend.integrations.social_feeds.REPORTS_DIR", reports_dir):
            snapshot = await monitor.fetch_feeds()
            monitor.generate_report("hourly", snapshot)
            monitor.generate_report("daily", snapshot)
            monitor.generate_report("weekly", snapshot)

            all_reports = monitor.list_reports()
            daily_only = monitor.list_reports(cadence="daily")
            weekly_only = monitor.list_reports(cadence="weekly")

        assert len(all_reports) == 3
        assert len(daily_only) == 1
        assert daily_only[0]["cadence"] == "daily"
        assert len(weekly_only) == 1
        assert weekly_only[0]["cadence"] == "weekly"

    def test_list_reports_empty(self, tmp_path):
        monitor = SocialFeedMonitor()
        with patch("backend.integrations.social_feeds.REPORTS_DIR", tmp_path / "nonexistent"):
            reports = monitor.list_reports()
        assert reports == []

    # -- Heartbeat tests --

    def test_heartbeat_not_active_by_default(self):
        monitor = SocialFeedMonitor()
        assert not monitor.heartbeat_active
        status = monitor.heartbeat_status()
        assert not status["hourly"]
        assert not status["daily"]
        assert not status["weekly"]

    def test_stop_heartbeat_when_not_running(self):
        monitor = SocialFeedMonitor()
        assert not monitor.stop_heartbeat("daily")

    def test_stop_all_heartbeats_when_not_running(self):
        monitor = SocialFeedMonitor()
        assert not monitor.stop_heartbeat()

    def test_start_heartbeat_invalid_cadence(self):
        monitor = SocialFeedMonitor()
        with pytest.raises(ValueError, match="Invalid cadence"):
            monitor.start_heartbeat("biweekly")

    # -- Chat handler tests --

    @pytest.mark.asyncio
    async def test_handle_chat_default(self):
        """Default chat handler returns overview with cadence options."""
        monitor = SocialFeedMonitor("Accelerate Okanagan")
        result = await monitor.handle_chat("hello", {})
        assert "Social Feed Monitor" in result
        assert "Accelerate Okanagan" in result
        assert "hourly" in result.lower()
        assert "daily" in result.lower()
        assert "weekly" in result.lower()

    @pytest.mark.asyncio
    async def test_handle_chat_daily_report(self, tmp_path):
        """Chat handler responds to daily report request."""
        monitor = SocialFeedMonitor("Test Co")
        with patch("backend.integrations.social_feeds.FEEDS_DIR", tmp_path), \
             patch("backend.integrations.social_feeds.TWITTER_FILE", tmp_path / "twitter.json"), \
             patch("backend.integrations.social_feeds.REVIEWS_FILE", tmp_path / "reviews.json"), \
             patch("backend.integrations.social_feeds.REPORTS_DIR", tmp_path / "reports"):
            result = await monitor.handle_chat("show me the daily report", {})

        assert "Daily Market Analysis" in result
        assert "Twitter Activity" in result
        assert "Google Reviews" in result

    @pytest.mark.asyncio
    async def test_handle_chat_hourly_report(self, tmp_path):
        """Chat handler responds to hourly report request."""
        monitor = SocialFeedMonitor("Test Co")
        with patch("backend.integrations.social_feeds.FEEDS_DIR", tmp_path), \
             patch("backend.integrations.social_feeds.TWITTER_FILE", tmp_path / "twitter.json"), \
             patch("backend.integrations.social_feeds.REVIEWS_FILE", tmp_path / "reviews.json"), \
             patch("backend.integrations.social_feeds.REPORTS_DIR", tmp_path / "reports"):
            result = await monitor.handle_chat("show hourly report", {})

        assert "Hourly Pulse" in result

    @pytest.mark.asyncio
    async def test_handle_chat_weekly_report(self, tmp_path):
        """Chat handler responds to weekly report request."""
        monitor = SocialFeedMonitor("Test Co")
        with patch("backend.integrations.social_feeds.FEEDS_DIR", tmp_path), \
             patch("backend.integrations.social_feeds.TWITTER_FILE", tmp_path / "twitter.json"), \
             patch("backend.integrations.social_feeds.REVIEWS_FILE", tmp_path / "reviews.json"), \
             patch("backend.integrations.social_feeds.REPORTS_DIR", tmp_path / "reports"):
            result = await monitor.handle_chat("show weekly report", {})

        assert "Weekly Strategic Digest" in result
        assert "Week-over-Week Comparison" in result
        assert "Strategic Recommendations" in result

    @pytest.mark.asyncio
    async def test_handle_chat_fetch(self, tmp_path):
        """Chat handler responds to fetch request."""
        monitor = SocialFeedMonitor("Test Co")
        with patch("backend.integrations.social_feeds.FEEDS_DIR", tmp_path), \
             patch("backend.integrations.social_feeds.TWITTER_FILE", tmp_path / "twitter.json"), \
             patch("backend.integrations.social_feeds.REVIEWS_FILE", tmp_path / "reviews.json"), \
             patch("backend.integrations.social_feeds.REPORTS_DIR", tmp_path / "reports"):
            result = await monitor.handle_chat("refresh feeds", {})

        assert "Social Feeds" in result
        assert "Twitter/X" in result
        assert "Google Reviews" in result

    # -- Cadence detection tests --

    def test_detect_cadence_hourly(self):
        monitor = SocialFeedMonitor()
        assert monitor._detect_cadence("show hourly report") == "hourly"
        assert monitor._detect_cadence("what happened this hour") == "hourly"

    def test_detect_cadence_weekly(self):
        monitor = SocialFeedMonitor()
        assert monitor._detect_cadence("show weekly digest") == "weekly"
        assert monitor._detect_cadence("what happened this week") == "weekly"

    def test_detect_cadence_daily_default(self):
        monitor = SocialFeedMonitor()
        assert monitor._detect_cadence("show report") == "daily"
        assert monitor._detect_cadence("daily analysis") == "daily"

    def test_detect_cadences_multi_all(self):
        monitor = SocialFeedMonitor()
        result = monitor._detect_cadences_multi("start all heartbeats")
        assert set(result) == {"hourly", "daily", "weekly"}

    def test_detect_cadences_multi_specific(self):
        monitor = SocialFeedMonitor()
        result = monitor._detect_cadences_multi("start hourly and weekly heartbeat")
        assert "hourly" in result
        assert "weekly" in result

    def test_detect_cadences_multi_default(self):
        monitor = SocialFeedMonitor()
        result = monitor._detect_cadences_multi("start heartbeat")
        assert result == ["daily"]
