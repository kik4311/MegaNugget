from html.parser import HTMLParser
from pathlib import Path
from urllib.parse import urlsplit
import unittest

ROOT = Path(__file__).resolve().parents[1]
SITE = ROOT / "apps/website"


class Page(HTMLParser):
    def __init__(self):
        super().__init__()
        self.elements = []

    def handle_starttag(self, tag, attrs):
        self.elements.append((tag, dict(attrs)))


class WebsiteTests(unittest.TestCase):
    def setUp(self):
        self.html = (SITE / "index.html").read_text()
        self.page = Page()
        self.page.feed(self.html)

    def test_assets_are_local_and_exist(self):
        for tag, attrs in self.page.elements:
            url = attrs.get("src")
            if tag == "link":
                url = attrs.get("href")
            if not url:
                continue
            with self.subTest(url=url):
                self.assertFalse(urlsplit(url).scheme)
                self.assertFalse(url.startswith("/"))
                self.assertTrue((SITE / url).is_file())

    def test_navigation_has_real_destinations(self):
        ids = {attrs["id"] for _, attrs in self.page.elements if "id" in attrs}
        for tag, attrs in self.page.elements:
            if tag != "a":
                continue
            href = attrs["href"]
            with self.subTest(href=href):
                if href == "#":
                    continue
                if href.startswith("#"):
                    self.assertIn(href[1:], ids)
                else:
                    self.assertEqual(urlsplit(href).scheme, "https")
                    self.assertEqual(urlsplit(href).netloc, "github.com")

    def test_no_credential_forms_or_trackers(self):
        self.assertFalse(any(tag in {"form", "input", "iframe"} for tag, _ in self.page.elements))
        self.assertIn("connect-src 'none'", self.html)
        self.assertIn('lang="ru"', self.html)
        self.assertIn('name="viewport"', self.html)
        self.assertIn("не скриншот", self.html)
        self.assertIn("Системные твики ещё в разработке", " ".join(self.html.split()))

    def test_same_logo_as_application(self):
        self.assertEqual((SITE / "assets/meganugget-logo.svg").read_bytes(),
                         (ROOT / "assets/branding/meganugget-logo.svg").read_bytes())


if __name__ == "__main__":
    unittest.main()
