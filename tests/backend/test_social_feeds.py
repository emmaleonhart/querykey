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
    DailyReport,
    _generate_demo_tweets,
    _generate_demo_reviews,
    _classify_sentiment,
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
        # Ensure JSON-serializable
        json.dumps(d)


class TestDailyReport:
    def test_to_dict(self):
        report = DailyReport(
            company="Test Co",
            report_date="2026-03-08",
            generated_at="2026-03-08T12:00:00",
            sentiment_trend="Mostly Positive",
            key_themes=["Innovation", "Support"],
            action_items=["Respond to negative reviews"],
            market_pulse="Looking good.",
        )
        d = report.to_dict()
        assert d["company"] == "Test Co"
        assert d["sentiment_trend"] == "Mostly Positive"
        assert len(d["key_themes"]) == 2
        json.dumps(d)


# ---------------------------------------------------------------------------
# Demo data generation tests
# ---------------------------------------------------------------------------

class TestDemoData:
    def test_generate_demo_tweets(self):
        tweets = _generate_demo_tweets()
        assert len(tweets) == 12
        assert all(isinstance(t, Tweet) for t in tweets)
        assert all(t.id.startswith("tw_") for t in tweets)
        # Should have a mix of sentiments
        sentiments = {t.sentiment for t in tweets}
        assert "positive" in sentiments
        assert "negative" in sentiments

    def test_generate_demo_reviews(self):
        reviews = _generate_demo_reviews()
        assert len(reviews) == 10
        assert all(isinstance(r, GoogleReview) for r in reviews)
        assert all(r.id.startswith("gr_") for r in reviews)
        # Ratings should be 1-5
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
        # Patch the storage paths to use temp dir
        with patch("backend.integrations.social_feeds.FEEDS_DIR", tmp_path), \
             patch("backend.integrations.social_feeds.TWITTER_FILE", tmp_path / "twitter.json"), \
             patch("backend.integrations.social_feeds.REVIEWS_FILE", tmp_path / "reviews.json"), \
             patch("backend.integrations.social_feeds.REPORTS_DIR", tmp_path / "reports"):
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

    @pytest.mark.asyncio
    async def test_generate_report(self, tmp_path):
        """Test report generation from feed data."""
        reports_dir = tmp_path / "reports"
        reports_dir.mkdir()

        monitor = SocialFeedMonitor("Test Co")
        with patch("backend.integrations.social_feeds.FEEDS_DIR", tmp_path), \
             patch("backend.integrations.social_feeds.TWITTER_FILE", tmp_path / "twitter.json"), \
             patch("backend.integrations.social_feeds.REVIEWS_FILE", tmp_path / "reviews.json"), \
             patch("backend.integrations.social_feeds.REPORTS_DIR", reports_dir):
            snapshot = await monitor.fetch_feeds()
            report = monitor.generate_report(snapshot)

        assert report.company == "Test Co"
        assert report.report_date
        assert report.sentiment_trend
        assert len(report.key_themes) > 0
        assert len(report.action_items) > 0
        assert "Test Co" in report.market_pulse

        # Check tweet summary
        assert report.tweet_summary["total_tweets"] == 12
        assert report.tweet_summary["total_engagement"] > 0

        # Check review summary
        assert report.review_summary["total_reviews"] == 10
        assert report.review_summary["average_rating"] > 0

    @pytest.mark.asyncio
    async def test_report_saved_to_disk(self, tmp_path):
        """Verify report is saved as JSON."""
        reports_dir = tmp_path / "reports"
        reports_dir.mkdir()

        monitor = SocialFeedMonitor("Test Co")
        with patch("backend.integrations.social_feeds.FEEDS_DIR", tmp_path), \
             patch("backend.integrations.social_feeds.TWITTER_FILE", tmp_path / "twitter.json"), \
             patch("backend.integrations.social_feeds.REVIEWS_FILE", tmp_path / "reviews.json"), \
             patch("backend.integrations.social_feeds.REPORTS_DIR", reports_dir):
            snapshot = await monitor.fetch_feeds()
            report = monitor.generate_report(snapshot)

        report_files = list(reports_dir.glob("daily_report_*.json"))
        assert len(report_files) == 1

        saved = json.loads(report_files[0].read_text())
        assert saved["company"] == "Test Co"
        assert "tweet_summary" in saved
        assert "review_summary" in saved

    def test_list_reports_empty(self, tmp_path):
        monitor = SocialFeedMonitor()
        with patch("backend.integrations.social_feeds.REPORTS_DIR", tmp_path / "nonexistent"):
            reports = monitor.list_reports()
        assert reports == []

    @pytest.mark.asyncio
    async def test_list_reports_with_data(self, tmp_path):
        reports_dir = tmp_path / "reports"
        reports_dir.mkdir()

        monitor = SocialFeedMonitor("Test Co")
        with patch("backend.integrations.social_feeds.FEEDS_DIR", tmp_path), \
             patch("backend.integrations.social_feeds.TWITTER_FILE", tmp_path / "twitter.json"), \
             patch("backend.integrations.social_feeds.REVIEWS_FILE", tmp_path / "reviews.json"), \
             patch("backend.integrations.social_feeds.REPORTS_DIR", reports_dir):
            snapshot = await monitor.fetch_feeds()
            monitor.generate_report(snapshot)
            reports = monitor.list_reports()

        assert len(reports) == 1
        assert "filename" in reports[0]
        assert "date" in reports[0]

    @pytest.mark.asyncio
    async def test_handle_chat_default(self):
        """Default chat handler returns overview."""
        monitor = SocialFeedMonitor("Accelerate Okanagan")
        result = await monitor.handle_chat("hello", {})
        assert "Social Feed Monitor" in result
        assert "Accelerate Okanagan" in result

    @pytest.mark.asyncio
    async def test_handle_chat_report(self, tmp_path):
        """Chat handler responds to report request."""
        reports_dir = tmp_path / "reports"
        reports_dir.mkdir()

        monitor = SocialFeedMonitor("Test Co")
        with patch("backend.integrations.social_feeds.FEEDS_DIR", tmp_path), \
             patch("backend.integrations.social_feeds.TWITTER_FILE", tmp_path / "twitter.json"), \
             patch("backend.integrations.social_feeds.REVIEWS_FILE", tmp_path / "reviews.json"), \
             patch("backend.integrations.social_feeds.REPORTS_DIR", reports_dir):
            result = await monitor.handle_chat("show me the report", {})

        assert "Daily Market Analysis" in result
        assert "Twitter Activity" in result
        assert "Google Reviews" in result

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

    def test_heartbeat_not_active_by_default(self):
        monitor = SocialFeedMonitor()
        assert not monitor.heartbeat_active

    def test_stop_heartbeat_when_not_running(self):
        monitor = SocialFeedMonitor()
        assert not monitor.stop_heartbeat()
