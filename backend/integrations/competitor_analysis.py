"""
Competitor Analysis & Blue Ocean Strategy Module

Provides web scraping-powered competitor research and Blue Ocean Strategy
analysis. This turns Sakuya Assistant from a data tool into a strategic
business consultant:

  1. Scrape public competitor information (websites, social media, reviews)
  2. Extract key competitive factors (pricing, features, channels, etc.)
  3. Build a Blue Ocean Strategy Canvas comparing competitors
  4. Identify uncontested market space and recommend strategic moves
  5. Generate actionable recommendations with the Four Actions Framework

The module uses httpx + BeautifulSoup for scraping and structures the
analysis around established Blue Ocean Strategy methodology.
"""

import asyncio
import json
import logging
import re
from dataclasses import dataclass, field, asdict
from datetime import datetime
from pathlib import Path
from typing import Any, Optional
from urllib.parse import urlparse

import httpx

try:
    from bs4 import BeautifulSoup
    HAS_BS4 = True
except ImportError:
    HAS_BS4 = False

logger = logging.getLogger("tojo.competitor_analysis")


# ---------------------------------------------------------------------------
# Data models
# ---------------------------------------------------------------------------

@dataclass
class CompetitorProfile:
    """Scraped and structured profile of a single competitor."""
    name: str
    url: Optional[str] = None
    description: Optional[str] = None
    tagline: Optional[str] = None
    products_services: list[str] = field(default_factory=list)
    key_features: list[str] = field(default_factory=list)
    pricing_info: Optional[str] = None
    target_audience: Optional[str] = None
    social_links: dict[str, str] = field(default_factory=dict)
    tech_stack_hints: list[str] = field(default_factory=list)
    meta_keywords: list[str] = field(default_factory=list)
    scraped_at: Optional[str] = None
    scrape_status: str = "pending"  # pending | success | partial | failed
    raw_text_sample: Optional[str] = None

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass
class CompetitiveFactor:
    """A single factor on the Blue Ocean Strategy Canvas."""
    name: str
    description: str = ""
    scores: dict[str, int] = field(default_factory=dict)  # competitor_name -> score (1-10)

    def to_dict(self) -> dict[str, Any]:
        return asdict(self)


@dataclass
class BlueOceanCanvas:
    """The Strategy Canvas: factors vs competitors with scores."""
    factors: list[CompetitiveFactor] = field(default_factory=list)
    competitors: list[str] = field(default_factory=list)
    your_company: Optional[str] = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "factors": [f.to_dict() for f in self.factors],
            "competitors": self.competitors,
            "your_company": self.your_company,
        }


@dataclass
class FourActionsFramework:
    """Blue Ocean Four Actions Framework recommendations."""
    eliminate: list[str] = field(default_factory=list)
    reduce: list[str] = field(default_factory=list)
    raise_factors: list[str] = field(default_factory=list)
    create: list[str] = field(default_factory=list)

    def to_dict(self) -> dict[str, Any]:
        return {
            "eliminate": self.eliminate,
            "reduce": self.reduce,
            "raise": self.raise_factors,
            "create": self.create,
        }


@dataclass
class CompetitorAnalysisReport:
    """Full competitor analysis report with Blue Ocean Strategy."""
    industry: str
    your_company: Optional[str] = None
    competitors: list[CompetitorProfile] = field(default_factory=list)
    canvas: Optional[BlueOceanCanvas] = None
    four_actions: Optional[FourActionsFramework] = None
    blue_ocean_opportunities: list[str] = field(default_factory=list)
    strategic_recommendations: list[str] = field(default_factory=list)
    generated_at: Optional[str] = None

    def to_dict(self) -> dict[str, Any]:
        return {
            "industry": self.industry,
            "your_company": self.your_company,
            "competitors": [c.to_dict() for c in self.competitors],
            "canvas": self.canvas.to_dict() if self.canvas else None,
            "four_actions": self.four_actions.to_dict() if self.four_actions else None,
            "blue_ocean_opportunities": self.blue_ocean_opportunities,
            "strategic_recommendations": self.strategic_recommendations,
            "generated_at": self.generated_at,
        }


# ---------------------------------------------------------------------------
# Default competitive factors by industry
# ---------------------------------------------------------------------------

INDUSTRY_FACTORS: dict[str, list[dict[str, str]]] = {
    "default": [
        {"name": "Price", "description": "Price point relative to market"},
        {"name": "Product Quality", "description": "Overall quality of product/service"},
        {"name": "Customer Service", "description": "Quality and responsiveness of support"},
        {"name": "Brand Recognition", "description": "Market awareness and reputation"},
        {"name": "Technology/Innovation", "description": "Use of cutting-edge technology"},
        {"name": "Ease of Use", "description": "Simplicity of user experience"},
        {"name": "Feature Breadth", "description": "Range of features offered"},
        {"name": "Market Reach", "description": "Geographic and demographic coverage"},
        {"name": "Customization", "description": "Ability to tailor to specific needs"},
        {"name": "Speed/Performance", "description": "Speed of delivery or performance"},
    ],
    "saas": [
        {"name": "Price", "description": "Subscription cost"},
        {"name": "Free Tier", "description": "Availability and generosity of free plan"},
        {"name": "Ease of Onboarding", "description": "Time to first value"},
        {"name": "API/Integrations", "description": "Integration ecosystem breadth"},
        {"name": "Uptime/Reliability", "description": "Service reliability"},
        {"name": "Customer Support", "description": "Support quality and channels"},
        {"name": "Feature Depth", "description": "Depth of core features"},
        {"name": "Data Security", "description": "Security certifications and practices"},
        {"name": "Scalability", "description": "Ability to grow with the customer"},
        {"name": "Mobile Experience", "description": "Mobile app quality"},
    ],
    "ecommerce": [
        {"name": "Price", "description": "Product pricing competitiveness"},
        {"name": "Product Range", "description": "Breadth of product catalog"},
        {"name": "Shipping Speed", "description": "Delivery time"},
        {"name": "Shipping Cost", "description": "Delivery pricing"},
        {"name": "Return Policy", "description": "Ease and generosity of returns"},
        {"name": "Website UX", "description": "Shopping experience quality"},
        {"name": "Customer Reviews", "description": "Social proof and ratings"},
        {"name": "Brand Trust", "description": "Consumer confidence"},
        {"name": "Loyalty Program", "description": "Rewards and retention programs"},
        {"name": "Sustainability", "description": "Environmental responsibility"},
    ],
    "consulting": [
        {"name": "Expertise Depth", "description": "Specialist knowledge"},
        {"name": "Price", "description": "Fee structure"},
        {"name": "Reputation", "description": "Market standing and references"},
        {"name": "Industry Focus", "description": "Vertical specialization"},
        {"name": "Team Size", "description": "Resource availability"},
        {"name": "Methodology", "description": "Structured approach quality"},
        {"name": "Deliverable Quality", "description": "Output quality and actionability"},
        {"name": "Communication", "description": "Client communication and reporting"},
        {"name": "Technology Use", "description": "Use of data/tech in consulting"},
        {"name": "Geographic Reach", "description": "Service area coverage"},
    ],
}

# Common social media domains for link extraction
SOCIAL_DOMAINS = {
    "twitter.com": "twitter",
    "x.com": "twitter",
    "facebook.com": "facebook",
    "linkedin.com": "linkedin",
    "instagram.com": "instagram",
    "youtube.com": "youtube",
    "github.com": "github",
    "tiktok.com": "tiktok",
}


# ---------------------------------------------------------------------------
# Web scraper
# ---------------------------------------------------------------------------

class CompetitorScraper:
    """Scrapes public competitor websites for business intelligence."""

    # Headers mimicking a standard browser to avoid basic bot blocks
    DEFAULT_HEADERS = {
        "User-Agent": (
            "Mozilla/5.0 (Windows NT 10.0; Win64; x64) "
            "AppleWebKit/537.36 (KHTML, like Gecko) "
            "Chrome/120.0.0.0 Safari/537.36"
        ),
        "Accept": "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
        "Accept-Language": "en-US,en;q=0.9",
    }

    def __init__(self, timeout: float = 15.0):
        self.timeout = timeout

    async def scrape_competitor(self, url: str, name: Optional[str] = None) -> CompetitorProfile:
        """
        Scrape a competitor's website and extract business-relevant info.

        Args:
            url: The competitor's website URL
            name: Optional company name (inferred from domain if not given)

        Returns:
            CompetitorProfile with extracted data
        """
        if not HAS_BS4:
            profile = CompetitorProfile(
                name=name or self._name_from_url(url),
                url=url,
                scrape_status="failed",
                description="beautifulsoup4 is not installed. Install it with: pip install beautifulsoup4",
            )
            return profile

        parsed = urlparse(url)
        if not parsed.scheme:
            url = f"https://{url}"

        if not name:
            name = self._name_from_url(url)

        profile = CompetitorProfile(name=name, url=url)

        try:
            async with httpx.AsyncClient(
                timeout=self.timeout,
                follow_redirects=True,
                headers=self.DEFAULT_HEADERS,
            ) as client:
                response = await client.get(url)
                response.raise_for_status()

            soup = BeautifulSoup(response.text, "html.parser")
            self._extract_metadata(soup, profile)
            self._extract_social_links(soup, url, profile)
            self._extract_features_and_products(soup, profile)
            self._extract_tech_hints(response.text, profile)
            self._extract_text_sample(soup, profile)

            profile.scraped_at = datetime.utcnow().isoformat()
            profile.scrape_status = "success"
            logger.info("Successfully scraped competitor: %s (%s)", name, url)

        except httpx.HTTPStatusError as exc:
            profile.scrape_status = "failed"
            profile.description = f"HTTP {exc.response.status_code}: could not access {url}"
            logger.warning("HTTP error scraping %s: %s", url, exc)

        except (httpx.ConnectError, httpx.TimeoutException) as exc:
            profile.scrape_status = "failed"
            profile.description = f"Connection failed: {exc}"
            logger.warning("Connection error scraping %s: %s", url, exc)

        except Exception as exc:
            profile.scrape_status = "partial"
            profile.description = f"Partial scrape - some data may be missing: {exc}"
            logger.warning("Error scraping %s: %s", url, exc)

        return profile

    async def scrape_multiple(self, competitors: list[dict[str, str]]) -> list[CompetitorProfile]:
        """
        Scrape multiple competitors concurrently.

        Args:
            competitors: List of dicts with 'url' and optional 'name' keys

        Returns:
            List of CompetitorProfile objects
        """
        tasks = [
            self.scrape_competitor(c["url"], c.get("name"))
            for c in competitors
        ]
        return await asyncio.gather(*tasks)

    # -- Extraction helpers --------------------------------------------------

    def _name_from_url(self, url: str) -> str:
        """Infer a company name from a URL."""
        parsed = urlparse(url)
        domain = parsed.netloc or parsed.path
        domain = domain.replace("www.", "")
        name = domain.split(".")[0]
        return name.replace("-", " ").replace("_", " ").title()

    def _extract_metadata(self, soup: BeautifulSoup, profile: CompetitorProfile) -> None:
        """Extract title, description, and keywords from <head> meta tags."""
        # Title
        title_tag = soup.find("title")
        if title_tag and title_tag.string:
            profile.tagline = title_tag.string.strip()

        # Meta description
        meta_desc = soup.find("meta", attrs={"name": re.compile(r"description", re.I)})
        if meta_desc and meta_desc.get("content"):
            profile.description = meta_desc["content"].strip()

        # OG description fallback
        if not profile.description:
            og_desc = soup.find("meta", attrs={"property": "og:description"})
            if og_desc and og_desc.get("content"):
                profile.description = og_desc["content"].strip()

        # Keywords
        meta_kw = soup.find("meta", attrs={"name": re.compile(r"keywords", re.I)})
        if meta_kw and meta_kw.get("content"):
            profile.meta_keywords = [
                kw.strip() for kw in meta_kw["content"].split(",") if kw.strip()
            ]

    def _extract_social_links(self, soup: BeautifulSoup, base_url: str, profile: CompetitorProfile) -> None:
        """Find social media links on the page."""
        for link in soup.find_all("a", href=True):
            href = link["href"]
            if not href.startswith("http"):
                continue
            for domain, platform in SOCIAL_DOMAINS.items():
                if domain in href:
                    profile.social_links[platform] = href
                    break

    def _extract_features_and_products(self, soup: BeautifulSoup, profile: CompetitorProfile) -> None:
        """Extract likely product/feature mentions from headings and lists."""
        # Look for feature-like sections
        feature_keywords = re.compile(
            r"feature|product|service|solution|benefit|offering|capability|what we do",
            re.I,
        )

        # Check headings
        for heading in soup.find_all(["h1", "h2", "h3", "h4"]):
            text = heading.get_text(strip=True)
            if feature_keywords.search(text):
                # Grab siblings or following list items
                sibling = heading.find_next_sibling()
                if sibling and sibling.name in ("ul", "ol"):
                    for li in sibling.find_all("li"):
                        item = li.get_text(strip=True)
                        if item and len(item) < 200:
                            profile.key_features.append(item)
                elif sibling and sibling.name == "p":
                    text = sibling.get_text(strip=True)
                    if text and len(text) < 300:
                        profile.products_services.append(text)

        # Also grab nav items that look like product names
        nav = soup.find("nav") or soup.find(attrs={"role": "navigation"})
        if nav:
            for link in nav.find_all("a"):
                text = link.get_text(strip=True)
                href = link.get("href", "")
                if text and len(text) < 50 and any(
                    kw in href.lower() for kw in ["product", "solution", "service", "feature"]
                ):
                    profile.products_services.append(text)

        # Deduplicate
        profile.key_features = list(dict.fromkeys(profile.key_features))[:15]
        profile.products_services = list(dict.fromkeys(profile.products_services))[:10]

    def _extract_tech_hints(self, html: str, profile: CompetitorProfile) -> None:
        """Look for technology stack hints in the HTML source."""
        tech_patterns = {
            "React": r"react(?:\.production|\.development|dom)",
            "Vue.js": r"vue(?:\.runtime|\.global|\.js)",
            "Angular": r"angular(?:\.min\.js|\.js|/core)",
            "Next.js": r"_next/",
            "WordPress": r"wp-content|wp-includes",
            "Shopify": r"cdn\.shopify\.com",
            "Squarespace": r"squarespace\.com",
            "HubSpot": r"hubspot\.com|hs-scripts",
            "Salesforce": r"force\.com|salesforce",
            "Intercom": r"intercom(?:cdn|\.io)",
            "Zendesk": r"zendesk\.com",
            "Google Analytics": r"google-analytics\.com|gtag",
            "Stripe": r"stripe\.com|js\.stripe",
            "Segment": r"segment\.(?:com|io)",
        }
        for tech, pattern in tech_patterns.items():
            if re.search(pattern, html, re.I):
                profile.tech_stack_hints.append(tech)

    def _extract_text_sample(self, soup: BeautifulSoup, profile: CompetitorProfile) -> None:
        """Get a clean text sample from the page body for LLM analysis."""
        # Remove script/style
        for tag in soup(["script", "style", "noscript", "header", "footer", "nav"]):
            tag.decompose()

        text = soup.get_text(separator=" ", strip=True)
        # Clean up whitespace
        text = re.sub(r"\s+", " ", text)
        # Take first ~1500 chars as a representative sample
        profile.raw_text_sample = text[:1500] if text else None


# ---------------------------------------------------------------------------
# Blue Ocean Strategy Analyzer
# ---------------------------------------------------------------------------

class BlueOceanAnalyzer:
    """
    Generates Blue Ocean Strategy analysis from competitor profiles.

    When OpenClaw/LLM is available, the analysis is routed through the AI
    for deeper insight. When not available, the analyzer uses rule-based
    heuristics to produce a useful (if less nuanced) report.
    """

    def __init__(self, industry: str = "default"):
        self.industry = industry.lower()

    def get_factors(self) -> list[CompetitiveFactor]:
        """Get the competitive factors for the configured industry."""
        factor_defs = INDUSTRY_FACTORS.get(self.industry, INDUSTRY_FACTORS["default"])
        return [CompetitiveFactor(name=f["name"], description=f["description"]) for f in factor_defs]

    def build_canvas(
        self,
        competitors: list[CompetitorProfile],
        your_company: Optional[str] = None,
        custom_scores: Optional[dict[str, dict[str, int]]] = None,
    ) -> BlueOceanCanvas:
        """
        Build a Strategy Canvas by scoring competitors on each factor.

        If custom_scores is provided, those are used directly.
        Otherwise, heuristic scoring is applied based on scraped data.

        Args:
            competitors: List of scraped competitor profiles
            your_company: Your company name (for the canvas)
            custom_scores: Optional dict of {competitor_name: {factor_name: score}}

        Returns:
            BlueOceanCanvas ready for visualization/export
        """
        factors = self.get_factors()
        comp_names = [c.name for c in competitors]

        canvas = BlueOceanCanvas(
            factors=factors,
            competitors=comp_names,
            your_company=your_company,
        )

        if custom_scores:
            for factor in canvas.factors:
                for comp_name in comp_names:
                    if comp_name in custom_scores and factor.name in custom_scores[comp_name]:
                        factor.scores[comp_name] = custom_scores[comp_name][factor.name]
        else:
            # Heuristic scoring based on scraped data
            for factor in canvas.factors:
                for comp in competitors:
                    factor.scores[comp.name] = self._heuristic_score(factor.name, comp)

        return canvas

    def generate_four_actions(self, canvas: BlueOceanCanvas) -> FourActionsFramework:
        """
        Apply the Four Actions Framework based on canvas data.

        Identifies factors to Eliminate, Reduce, Raise, or Create
        to differentiate from the competition.
        """
        four_actions = FourActionsFramework()

        if not canvas.factors or not canvas.competitors:
            return four_actions

        for factor in canvas.factors:
            scores = list(factor.scores.values())
            if not scores:
                continue
            avg = sum(scores) / len(scores)

            # All competitors score high -> consider eliminating or reducing
            # (it's table stakes, not a differentiator)
            if avg >= 7:
                four_actions.reduce.append(
                    f"{factor.name}: All competitors invest heavily here (avg {avg:.1f}/10). "
                    f"Consider whether you can reduce investment while maintaining adequacy."
                )
            # All competitors score low -> opportunity to raise
            elif avg <= 3:
                four_actions.raise_factors.append(
                    f"{factor.name}: Competitors underperform here (avg {avg:.1f}/10). "
                    f"Raising your level here could be a strong differentiator."
                )
            # Wide spread -> identify which end to target
            elif max(scores) - min(scores) >= 4:
                four_actions.raise_factors.append(
                    f"{factor.name}: Wide competitive spread ({min(scores)}-{max(scores)}). "
                    f"Opportunity to differentiate by excelling here."
                )

        # Suggest factors to create (not on any competitor's radar)
        four_actions.create.extend([
            "AI-powered insights: Use AI to deliver personalized strategic recommendations",
            "Automated monitoring: Continuous competitor tracking with alert triggers",
            "Collaborative strategy: Real-time strategy canvas sharing with team members",
        ])

        # Suggest factors to eliminate (industry conventions that waste resources)
        four_actions.eliminate.extend([
            "Consider eliminating features that competitors offer but customers rarely use",
            "Review overhead costs tied to matching competitor offerings that don't drive value",
        ])

        return four_actions

    def identify_blue_ocean_opportunities(
        self,
        canvas: BlueOceanCanvas,
        competitors: list[CompetitorProfile],
    ) -> list[str]:
        """
        Identify uncontested market spaces based on the canvas analysis.
        """
        opportunities = []

        # Look for factors where ALL competitors are weak
        for factor in canvas.factors:
            scores = list(factor.scores.values())
            if scores and max(scores) <= 4:
                opportunities.append(
                    f"**{factor.name}** is underserved across all competitors "
                    f"(max score: {max(scores)}/10). Excelling here opens uncontested space."
                )

        # Look for market gaps based on scraped data
        all_features = set()
        for comp in competitors:
            all_features.update(f.lower() for f in comp.key_features)

        if len(competitors) >= 2:
            # Features only one competitor has -> potential differentiation areas
            for comp in competitors:
                unique = [
                    f for f in comp.key_features
                    if sum(1 for c2 in competitors if f.lower() in [x.lower() for x in c2.key_features]) == 1
                ]
                if unique:
                    opportunities.append(
                        f"**{comp.name}** is the only competitor offering: {', '.join(unique[:3])}. "
                        f"Consider whether these represent valuable differentiators to adopt or avoid."
                    )

        # Default strategic suggestions based on scraped intelligence
        if not opportunities:
            opportunities.append(
                "No clear gaps found from automated analysis alone. "
                "Consider using the chat interface with OpenClaw connected for "
                "deeper AI-powered analysis of competitor positioning."
            )

        return opportunities

    def generate_recommendations(
        self,
        canvas: BlueOceanCanvas,
        four_actions: FourActionsFramework,
        competitors: list[CompetitorProfile],
    ) -> list[str]:
        """Generate strategic recommendations from the complete analysis."""
        recs = []

        # Pricing insight
        pricing_mentioned = [c for c in competitors if c.pricing_info]
        if pricing_mentioned:
            recs.append(
                f"**Pricing intelligence**: {len(pricing_mentioned)} of {len(competitors)} "
                f"competitors have visible pricing. Review their models to inform your strategy."
            )

        # Tech stack insight
        tech_counts: dict[str, int] = {}
        for comp in competitors:
            for tech in comp.tech_stack_hints:
                tech_counts[tech] = tech_counts.get(tech, 0) + 1
        if tech_counts:
            common_tech = sorted(tech_counts.items(), key=lambda x: -x[1])[:3]
            tech_str = ", ".join(f"{t[0]} ({t[1]} competitors)" for t in common_tech)
            recs.append(
                f"**Technology landscape**: Common technologies among competitors: {tech_str}. "
                f"Consider whether aligning or differentiating on tech stack creates advantage."
            )

        # Social presence insight
        social_counts: dict[str, int] = {}
        for comp in competitors:
            for platform in comp.social_links:
                social_counts[platform] = social_counts.get(platform, 0) + 1
        if social_counts:
            missing_socials = [
                p for p in ["twitter", "linkedin", "instagram"]
                if social_counts.get(p, 0) < len(competitors) / 2
            ]
            if missing_socials:
                recs.append(
                    f"**Social media gap**: Platforms underutilized by competitors: "
                    f"{', '.join(missing_socials)}. These could be low-competition channels."
                )

        # Four Actions summary
        if four_actions.raise_factors:
            recs.append(
                f"**Key differentiators**: {len(four_actions.raise_factors)} factors identified "
                f"where raising your performance would create clear competitive advantage."
            )

        if not recs:
            recs.append(
                "Connect OpenClaw for deeper AI-powered strategic recommendations "
                "based on your specific business context."
            )

        return recs

    def _heuristic_score(self, factor_name: str, competitor: CompetitorProfile) -> int:
        """
        Assign a heuristic score (1-10) for a factor based on scraped data.

        This is a rough approximation. For accurate scoring, users should
        provide custom scores or use the AI-assisted analysis.
        """
        base_score = 5  # Default middle score

        if competitor.scrape_status == "failed":
            return 3  # Low confidence for failed scrapes

        factor_lower = factor_name.lower()

        # Adjust based on available data
        if "price" in factor_lower:
            if competitor.pricing_info:
                base_score = 6  # They at least publish pricing (transparency)
            else:
                base_score = 5  # Unknown

        elif "feature" in factor_lower or "product" in factor_lower:
            feature_count = len(competitor.key_features) + len(competitor.products_services)
            if feature_count > 10:
                base_score = 8
            elif feature_count > 5:
                base_score = 6
            elif feature_count > 0:
                base_score = 4
            else:
                base_score = 3

        elif "technology" in factor_lower or "innovation" in factor_lower:
            tech_count = len(competitor.tech_stack_hints)
            if tech_count >= 5:
                base_score = 8
            elif tech_count >= 3:
                base_score = 6
            else:
                base_score = 4

        elif "brand" in factor_lower or "recognition" in factor_lower:
            # More social presence = more brand recognition
            social_count = len(competitor.social_links)
            if social_count >= 4:
                base_score = 8
            elif social_count >= 2:
                base_score = 6
            else:
                base_score = 4

        elif "ease" in factor_lower or "usability" in factor_lower or "use" in factor_lower:
            # Hard to determine from scraping alone
            base_score = 5

        elif "customer" in factor_lower or "support" in factor_lower or "service" in factor_lower:
            # Check for support-related social/tech hints
            has_support_tech = any(
                t in ["Intercom", "Zendesk"] for t in competitor.tech_stack_hints
            )
            base_score = 7 if has_support_tech else 5

        return min(max(base_score, 1), 10)


# ---------------------------------------------------------------------------
# Main orchestrator
# ---------------------------------------------------------------------------

class CompetitorAnalysis:
    """
    High-level orchestrator for competitor analysis workflows.

    Usage:
        analysis = CompetitorAnalysis()
        report = await analysis.analyze(
            industry="saas",
            competitors=[
                {"name": "Acme Corp", "url": "https://acme.com"},
                {"name": "Rival Inc", "url": "https://rival.io"},
            ],
            your_company="My Startup"
        )
    """

    def __init__(self):
        self.scraper = CompetitorScraper()
        self._reports: list[CompetitorAnalysisReport] = []
        self._storage_dir = Path.home() / ".tojo" / "competitor_reports"

    async def analyze(
        self,
        industry: str,
        competitors: list[dict[str, str]],
        your_company: Optional[str] = None,
        custom_scores: Optional[dict[str, dict[str, int]]] = None,
    ) -> CompetitorAnalysisReport:
        """
        Run a full competitor analysis with Blue Ocean Strategy.

        Args:
            industry: Industry type (saas, ecommerce, consulting, or default)
            competitors: List of dicts with 'url' and optional 'name'
            your_company: Your company name
            custom_scores: Optional manual scores {competitor: {factor: score}}

        Returns:
            CompetitorAnalysisReport with full analysis
        """
        logger.info(
            "Starting competitor analysis for %s industry with %d competitors",
            industry, len(competitors),
        )

        # 1. Scrape all competitors
        profiles = await self.scraper.scrape_multiple(competitors)

        # 2. Build Blue Ocean analysis
        analyzer = BlueOceanAnalyzer(industry)
        canvas = analyzer.build_canvas(profiles, your_company, custom_scores)
        four_actions = analyzer.generate_four_actions(canvas)
        opportunities = analyzer.identify_blue_ocean_opportunities(canvas, profiles)
        recommendations = analyzer.generate_recommendations(canvas, four_actions, profiles)

        # 3. Assemble report
        report = CompetitorAnalysisReport(
            industry=industry,
            your_company=your_company,
            competitors=profiles,
            canvas=canvas,
            four_actions=four_actions,
            blue_ocean_opportunities=opportunities,
            strategic_recommendations=recommendations,
            generated_at=datetime.utcnow().isoformat(),
        )

        self._reports.append(report)
        return report

    async def scrape_single(self, url: str, name: Optional[str] = None) -> CompetitorProfile:
        """Scrape a single competitor for quick preview."""
        return await self.scraper.scrape_competitor(url, name)

    def get_industry_factors(self, industry: str = "default") -> list[dict[str, str]]:
        """Return available competitive factors for an industry."""
        return INDUSTRY_FACTORS.get(industry.lower(), INDUSTRY_FACTORS["default"])

    def list_industries(self) -> list[str]:
        """Return list of supported industry types."""
        return list(INDUSTRY_FACTORS.keys())

    def save_report(self, report: CompetitorAnalysisReport) -> Path:
        """Save a report to disk as JSON."""
        self._storage_dir.mkdir(parents=True, exist_ok=True)
        timestamp = datetime.utcnow().strftime("%Y%m%d_%H%M%S")
        filename = f"competitor_analysis_{report.industry}_{timestamp}.json"
        path = self._storage_dir / filename
        path.write_text(json.dumps(report.to_dict(), indent=2, default=str))
        logger.info("Report saved to %s", path)
        return path

    def list_reports(self) -> list[dict[str, str]]:
        """List previously saved reports."""
        if not self._storage_dir.exists():
            return []
        reports = []
        for p in sorted(self._storage_dir.glob("competitor_analysis_*.json"), reverse=True):
            reports.append({
                "filename": p.name,
                "path": str(p),
                "modified": datetime.fromtimestamp(p.stat().st_mtime).isoformat(),
            })
        return reports

    def load_report(self, path: str) -> dict[str, Any]:
        """Load a previously saved report."""
        return json.loads(Path(path).read_text())

    def format_report_text(self, report: CompetitorAnalysisReport) -> str:
        """Format a report as human-readable text for chat display."""
        lines = []
        lines.append(f"## Competitor Analysis Report - {report.industry.title()} Industry")
        if report.your_company:
            lines.append(f"**Your Company:** {report.your_company}")
        lines.append(f"**Generated:** {report.generated_at}")
        lines.append("")

        # Competitors overview
        lines.append("### Competitors Analyzed")
        for comp in report.competitors:
            status_icon = "OK" if comp.scrape_status == "success" else "WARN"
            lines.append(f"- **{comp.name}** ({comp.url}) [{status_icon}]")
            if comp.description:
                lines.append(f"  {comp.description[:150]}")
            if comp.key_features:
                lines.append(f"  Key features: {', '.join(comp.key_features[:5])}")
        lines.append("")

        # Strategy Canvas
        if report.canvas:
            lines.append("### Strategy Canvas")
            lines.append("| Factor | " + " | ".join(report.canvas.competitors) + " |")
            lines.append("|" + "---|" * (len(report.canvas.competitors) + 1))
            for factor in report.canvas.factors:
                scores = [str(factor.scores.get(c, "-")) for c in report.canvas.competitors]
                lines.append(f"| {factor.name} | " + " | ".join(scores) + " |")
            lines.append("")

        # Four Actions Framework
        if report.four_actions:
            lines.append("### Four Actions Framework")
            if report.four_actions.eliminate:
                lines.append("**ELIMINATE:**")
                for item in report.four_actions.eliminate:
                    lines.append(f"- {item}")
            if report.four_actions.reduce:
                lines.append("**REDUCE:**")
                for item in report.four_actions.reduce:
                    lines.append(f"- {item}")
            if report.four_actions.raise_factors:
                lines.append("**RAISE:**")
                for item in report.four_actions.raise_factors:
                    lines.append(f"- {item}")
            if report.four_actions.create:
                lines.append("**CREATE:**")
                for item in report.four_actions.create:
                    lines.append(f"- {item}")
            lines.append("")

        # Blue Ocean Opportunities
        if report.blue_ocean_opportunities:
            lines.append("### Blue Ocean Opportunities")
            for opp in report.blue_ocean_opportunities:
                lines.append(f"- {opp}")
            lines.append("")

        # Strategic Recommendations
        if report.strategic_recommendations:
            lines.append("### Strategic Recommendations")
            for rec in report.strategic_recommendations:
                lines.append(f"- {rec}")

        return "\n".join(lines)
