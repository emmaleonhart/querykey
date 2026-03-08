"""Tests for the competitor analysis and Blue Ocean Strategy module."""

import json
import pytest
from unittest.mock import AsyncMock, patch, MagicMock

from backend.integrations.competitor_analysis import (
    CompetitorAnalysis,
    CompetitorProfile,
    CompetitorScraper,
    BlueOceanAnalyzer,
    BlueOceanCanvas,
    CompetitiveFactor,
    FourActionsFramework,
    CompetitorAnalysisReport,
    INDUSTRY_FACTORS,
)


# ---------------------------------------------------------------------------
# CompetitorProfile tests
# ---------------------------------------------------------------------------

class TestCompetitorProfile:
    def test_to_dict(self):
        profile = CompetitorProfile(
            name="Acme Corp",
            url="https://acme.com",
            description="A test company",
            key_features=["Feature A", "Feature B"],
            social_links={"twitter": "https://twitter.com/acme"},
        )
        d = profile.to_dict()
        assert d["name"] == "Acme Corp"
        assert d["url"] == "https://acme.com"
        assert d["description"] == "A test company"
        assert len(d["key_features"]) == 2
        assert "twitter" in d["social_links"]

    def test_default_values(self):
        profile = CompetitorProfile(name="Test")
        assert profile.scrape_status == "pending"
        assert profile.products_services == []
        assert profile.key_features == []
        assert profile.social_links == {}


# ---------------------------------------------------------------------------
# BlueOceanCanvas tests
# ---------------------------------------------------------------------------

class TestBlueOceanCanvas:
    def test_to_dict(self):
        canvas = BlueOceanCanvas(
            factors=[CompetitiveFactor(name="Price", scores={"A": 5, "B": 7})],
            competitors=["A", "B"],
            your_company="My Co",
        )
        d = canvas.to_dict()
        assert d["your_company"] == "My Co"
        assert len(d["factors"]) == 1
        assert d["factors"][0]["scores"]["A"] == 5


class TestFourActionsFramework:
    def test_to_dict(self):
        fa = FourActionsFramework(
            eliminate=["Legacy feature"],
            reduce=["Over-engineering"],
            raise_factors=["Customer support"],
            create=["AI insights"],
        )
        d = fa.to_dict()
        assert "eliminate" in d
        assert "reduce" in d
        assert "raise" in d
        assert "create" in d
        assert d["raise"] == ["Customer support"]


# ---------------------------------------------------------------------------
# BlueOceanAnalyzer tests
# ---------------------------------------------------------------------------

class TestBlueOceanAnalyzer:
    def test_get_factors_default(self):
        analyzer = BlueOceanAnalyzer("default")
        factors = analyzer.get_factors()
        assert len(factors) == 10
        assert factors[0].name == "Price"

    def test_get_factors_saas(self):
        analyzer = BlueOceanAnalyzer("saas")
        factors = analyzer.get_factors()
        assert len(factors) == 10
        assert any(f.name == "API/Integrations" for f in factors)

    def test_get_factors_unknown_falls_back(self):
        analyzer = BlueOceanAnalyzer("unknown_industry")
        factors = analyzer.get_factors()
        # Falls back to default
        assert len(factors) == 10

    def test_build_canvas_with_custom_scores(self):
        analyzer = BlueOceanAnalyzer()
        profiles = [
            CompetitorProfile(name="CompA", scrape_status="success"),
            CompetitorProfile(name="CompB", scrape_status="success"),
        ]
        custom_scores = {
            "CompA": {"Price": 8, "Product Quality": 6},
            "CompB": {"Price": 5, "Product Quality": 9},
        }
        canvas = analyzer.build_canvas(profiles, "My Co", custom_scores)
        assert canvas.your_company == "My Co"
        assert len(canvas.competitors) == 2

        price_factor = next(f for f in canvas.factors if f.name == "Price")
        assert price_factor.scores["CompA"] == 8
        assert price_factor.scores["CompB"] == 5

    def test_build_canvas_heuristic_scoring(self):
        analyzer = BlueOceanAnalyzer()
        profiles = [
            CompetitorProfile(
                name="TechCo",
                scrape_status="success",
                key_features=["AI", "ML", "Cloud", "API", "Mobile", "Desktop"],
                tech_stack_hints=["React", "Next.js", "Google Analytics"],
                social_links={"twitter": "x", "linkedin": "y", "github": "z", "facebook": "w"},
            ),
        ]
        canvas = analyzer.build_canvas(profiles)
        # TechCo has many features -> Feature Breadth score should be higher
        feature_factor = next(f for f in canvas.factors if f.name == "Feature Breadth")
        assert feature_factor.scores["TechCo"] >= 6

    def test_build_canvas_failed_scrape_low_score(self):
        analyzer = BlueOceanAnalyzer()
        profiles = [CompetitorProfile(name="FailedCo", scrape_status="failed")]
        canvas = analyzer.build_canvas(profiles)
        for factor in canvas.factors:
            assert factor.scores["FailedCo"] == 3

    def test_generate_four_actions(self):
        analyzer = BlueOceanAnalyzer()
        canvas = BlueOceanCanvas(
            factors=[
                CompetitiveFactor(name="Price", scores={"A": 8, "B": 9}),  # High avg -> reduce
                CompetitiveFactor(name="Innovation", scores={"A": 2, "B": 3}),  # Low avg -> raise
                CompetitiveFactor(name="Support", scores={"A": 3, "B": 8}),  # Wide spread -> raise
            ],
            competitors=["A", "B"],
        )
        four_actions = analyzer.generate_four_actions(canvas)
        assert len(four_actions.reduce) >= 1
        assert len(four_actions.raise_factors) >= 1
        assert len(four_actions.create) >= 1
        assert len(four_actions.eliminate) >= 1

    def test_identify_opportunities(self):
        analyzer = BlueOceanAnalyzer()
        canvas = BlueOceanCanvas(
            factors=[
                CompetitiveFactor(name="AI Features", scores={"A": 2, "B": 3}),
            ],
            competitors=["A", "B"],
        )
        profiles = [
            CompetitorProfile(name="A", key_features=["Basic"]),
            CompetitorProfile(name="B", key_features=["Advanced"]),
        ]
        opps = analyzer.identify_blue_ocean_opportunities(canvas, profiles)
        assert len(opps) >= 1
        # AI Features is underserved
        assert any("AI Features" in o for o in opps)

    def test_generate_recommendations(self):
        analyzer = BlueOceanAnalyzer()
        canvas = BlueOceanCanvas(factors=[], competitors=[])
        four_actions = FourActionsFramework(raise_factors=["Something"])
        profiles = [
            CompetitorProfile(
                name="TechCo",
                tech_stack_hints=["React", "HubSpot"],
                social_links={"twitter": "x"},
            ),
        ]
        recs = analyzer.generate_recommendations(canvas, four_actions, profiles)
        assert len(recs) >= 1


# ---------------------------------------------------------------------------
# CompetitorScraper tests
# ---------------------------------------------------------------------------

class TestCompetitorScraper:
    def test_name_from_url(self):
        scraper = CompetitorScraper()
        assert scraper._name_from_url("https://www.acme-corp.com") == "Acme Corp"
        assert scraper._name_from_url("https://example.io") == "Example"
        assert scraper._name_from_url("rival.com") == "Rival"

    @pytest.mark.asyncio
    async def test_scrape_competitor_connection_error(self):
        scraper = CompetitorScraper(timeout=1.0)
        profile = await scraper.scrape_competitor(
            "https://this-domain-definitely-does-not-exist-xyz123.com",
            "FakeCo",
        )
        assert profile.name == "FakeCo"
        assert profile.scrape_status in ("failed", "partial")

    @pytest.mark.asyncio
    async def test_scrape_competitor_adds_https(self):
        """Verify that URLs without scheme get https:// prepended."""
        scraper = CompetitorScraper(timeout=1.0)
        profile = await scraper.scrape_competitor(
            "this-also-doesnt-exist-abc789.com",
            "NoCo",
        )
        assert profile.url == "https://this-also-doesnt-exist-abc789.com"
        assert profile.scrape_status in ("failed", "partial")


# ---------------------------------------------------------------------------
# CompetitorAnalysis orchestrator tests
# ---------------------------------------------------------------------------

class TestCompetitorAnalysis:
    def test_list_industries(self):
        analysis = CompetitorAnalysis()
        industries = analysis.list_industries()
        assert "default" in industries
        assert "saas" in industries
        assert "ecommerce" in industries
        assert "consulting" in industries

    def test_get_industry_factors(self):
        analysis = CompetitorAnalysis()
        factors = analysis.get_industry_factors("saas")
        assert len(factors) == 10
        assert any(f["name"] == "Free Tier" for f in factors)

    def test_get_industry_factors_fallback(self):
        analysis = CompetitorAnalysis()
        factors = analysis.get_industry_factors("nonexistent")
        # Should fall back to default
        assert len(factors) == 10

    @pytest.mark.asyncio
    async def test_analyze_with_mock_scraping(self):
        """Test the full analysis pipeline with mocked scraping."""
        analysis = CompetitorAnalysis()

        mock_profiles = [
            CompetitorProfile(
                name="Competitor A",
                url="https://comp-a.com",
                description="A leading provider",
                key_features=["Feature 1", "Feature 2"],
                scrape_status="success",
            ),
            CompetitorProfile(
                name="Competitor B",
                url="https://comp-b.com",
                description="Another provider",
                key_features=["Feature 3"],
                social_links={"linkedin": "https://linkedin.com/comp-b"},
                scrape_status="success",
            ),
        ]

        with patch.object(analysis.scraper, "scrape_multiple", new_callable=AsyncMock) as mock_scrape:
            mock_scrape.return_value = mock_profiles

            report = await analysis.analyze(
                industry="saas",
                competitors=[
                    {"url": "https://comp-a.com", "name": "Competitor A"},
                    {"url": "https://comp-b.com", "name": "Competitor B"},
                ],
                your_company="My Startup",
            )

        assert report.industry == "saas"
        assert report.your_company == "My Startup"
        assert len(report.competitors) == 2
        assert report.canvas is not None
        assert report.four_actions is not None
        assert len(report.blue_ocean_opportunities) >= 1
        assert report.generated_at is not None

    @pytest.mark.asyncio
    async def test_analyze_report_to_dict(self):
        """Verify report serializes cleanly."""
        analysis = CompetitorAnalysis()

        mock_profiles = [
            CompetitorProfile(name="A", scrape_status="success"),
        ]

        with patch.object(analysis.scraper, "scrape_multiple", new_callable=AsyncMock) as mock_scrape:
            mock_scrape.return_value = mock_profiles
            report = await analysis.analyze(
                industry="default",
                competitors=[{"url": "https://a.com"}],
            )

        d = report.to_dict()
        assert "industry" in d
        assert "competitors" in d
        assert "canvas" in d
        assert "four_actions" in d
        assert "blue_ocean_opportunities" in d
        # Ensure JSON-serializable
        json.dumps(d)

    @pytest.mark.asyncio
    async def test_format_report_text(self):
        """Verify text formatting produces readable output."""
        analysis = CompetitorAnalysis()

        mock_profiles = [
            CompetitorProfile(
                name="Rival",
                url="https://rival.io",
                description="A fierce rival",
                key_features=["Speed", "Scale"],
                scrape_status="success",
            ),
        ]

        with patch.object(analysis.scraper, "scrape_multiple", new_callable=AsyncMock) as mock_scrape:
            mock_scrape.return_value = mock_profiles
            report = await analysis.analyze(
                industry="saas",
                competitors=[{"url": "https://rival.io", "name": "Rival"}],
                your_company="Our Co",
            )

        text = analysis.format_report_text(report)
        assert "Competitor Analysis Report" in text
        assert "Our Co" in text
        assert "Rival" in text
        assert "Strategy Canvas" in text
        assert "Four Actions Framework" in text

    def test_list_reports_empty(self):
        analysis = CompetitorAnalysis()
        reports = analysis.list_reports()
        assert isinstance(reports, list)


# ---------------------------------------------------------------------------
# Industry factor coverage
# ---------------------------------------------------------------------------

class TestIndustryFactors:
    def test_all_industries_have_10_factors(self):
        for industry, factors in INDUSTRY_FACTORS.items():
            assert len(factors) == 10, f"{industry} has {len(factors)} factors, expected 10"

    def test_all_factors_have_required_keys(self):
        for industry, factors in INDUSTRY_FACTORS.items():
            for f in factors:
                assert "name" in f, f"Missing 'name' in {industry} factor"
                assert "description" in f, f"Missing 'description' in {industry} factor"
