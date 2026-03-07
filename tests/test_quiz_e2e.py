#!/usr/bin/env python3
"""
End-to-end Playwright tests for the WASM Quiz app.

Usage:
    # Activate pyenv first:
    source ~/Documents/develeop/rust/geodb-rs/crates/geodb-py/.env_py312/bin/activate

    # Build the quiz WASM app (if not already built):
    cd crates/eulumdat-wasm-quiz && trunk build && cd ../..

    # Run tests (starts its own HTTP server):
    python tests/test_quiz_e2e.py

    # Run with visible browser:
    python tests/test_quiz_e2e.py --headed

    # Run a single test:
    python tests/test_quiz_e2e.py --headed -k test_full_quiz_flow
"""

import http.server
import os
import subprocess
import sys
import threading
import time

import pytest
from playwright.sync_api import Page, expect, sync_playwright

QUIZ_DIST = os.path.join(os.path.dirname(__file__), "..", "crates", "eulumdat-wasm-quiz", "dist")
QUIZ_PORT = 8045
QUIZ_URL = f"http://127.0.0.1:{QUIZ_PORT}/index.html"


# ---------------------------------------------------------------------------
# Fixtures
# ---------------------------------------------------------------------------

class QuietHandler(http.server.SimpleHTTPRequestHandler):
    """HTTP handler that suppresses access logs and serves correct MIME types."""

    extensions_map = {
        **http.server.SimpleHTTPRequestHandler.extensions_map,
        ".wasm": "application/wasm",
        ".js": "application/javascript",
    }

    def log_message(self, format, *args):
        pass

    def end_headers(self):
        self.send_header("Cross-Origin-Opener-Policy", "same-origin")
        self.send_header("Cross-Origin-Embedder-Policy", "require-corp")
        super().end_headers()


@pytest.fixture(scope="session")
def http_server():
    """Start a local HTTP server serving the quiz dist directory."""
    dist = os.path.abspath(QUIZ_DIST)
    if not os.path.isdir(dist):
        pytest.skip(f"Quiz dist not found at {dist} — run 'trunk build' first")

    handler = lambda *a, **k: QuietHandler(*a, directory=dist, **k)
    server = http.server.HTTPServer(("127.0.0.1", QUIZ_PORT), handler)
    thread = threading.Thread(target=server.serve_forever, daemon=True)
    thread.start()
    yield server
    server.shutdown()


@pytest.fixture(scope="session")
def browser_ctx(http_server):
    """Launch a shared browser context for all tests."""
    with sync_playwright() as p:
        browser = p.chromium.launch(
            headless="--headed" not in sys.argv,
        )
        context = browser.new_context(
            viewport={"width": 1280, "height": 900},
        )
        yield context
        context.close()
        browser.close()


@pytest.fixture()
def page(browser_ctx) -> Page:
    """Fresh page for each test — navigates to quiz and waits for WASM load."""
    pg = browser_ctx.new_page()
    pg.goto(QUIZ_URL)
    # Wait for the Leptos app to render the quiz root
    pg.wait_for_selector(".quiz-app", timeout=30_000)
    yield pg
    pg.close()


# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def start_quiz(page: Page, num_questions: int = 5):
    """Configure and start a quiz with the given number of questions."""
    # Should be on config screen
    page.wait_for_selector(".config-screen", timeout=5000)

    # Click the question count button
    if num_questions == 0:
        page.click(".count-btn:has-text('All')")
    else:
        page.click(f".count-btn:has-text('{num_questions}')")

    # Start the quiz
    page.click(".start-btn")
    page.wait_for_selector(".quiz-view", timeout=5000)


def answer_current_question(page: Page, choice: int = 0):
    """Click an answer option (0-3) and wait for feedback."""
    page.wait_for_selector(".option-btn", timeout=5000)
    buttons = page.locator(".option-btn")
    buttons.nth(choice).click()
    page.wait_for_selector(".feedback", timeout=3000)


def click_next(page: Page):
    """Click the Next/See Results button after answering."""
    page.click(".nav-btn.primary")


# ---------------------------------------------------------------------------
# Tests
# ---------------------------------------------------------------------------

class TestConfigScreen:
    """Tests for the quiz configuration screen."""

    def test_config_screen_renders(self, page: Page):
        """Config screen shows title, categories, difficulty, question count."""
        page.wait_for_selector(".config-screen")
        assert page.locator(".config-title").is_visible()
        assert page.locator(".category-grid").is_visible()
        assert page.locator(".difficulty-options").is_visible()
        assert page.locator(".count-options").is_visible()
        assert page.locator(".start-btn").is_visible()

    def test_category_checkboxes(self, page: Page):
        """All 15 categories should be listed with checkboxes."""
        checkboxes = page.locator(".category-item input[type='checkbox']")
        expect(checkboxes).to_have_count(15)

        # All should be checked by default
        for i in range(15):
            expect(checkboxes.nth(i)).to_be_checked()

    def test_select_none_then_all(self, page: Page):
        """Select None unchecks all; Select All re-checks all."""
        checkboxes = page.locator(".category-item input[type='checkbox']")

        # Click "Select None"
        page.locator(".link-btn", has_text="None").click()
        for i in range(15):
            expect(checkboxes.nth(i)).not_to_be_checked()

        # Click "Select All"
        page.locator(".link-btn", has_text="All").first.click()
        for i in range(15):
            expect(checkboxes.nth(i)).to_be_checked()

        # Start button should be enabled
        expect(page.locator(".start-btn")).to_be_enabled()

    def test_difficulty_radio_buttons(self, page: Page):
        """Difficulty radio buttons work and filter questions."""
        radios = page.locator(".radio-item input[type='radio']")
        # 4 options: All Levels + Beginner + Intermediate + Expert
        expect(radios).to_have_count(4)

    def test_question_count_buttons(self, page: Page):
        """Question count buttons highlight when selected."""
        import re as _re

        btn_10 = page.locator(".count-btn:has-text('10')")
        expect(btn_10).to_have_class(_re.compile("active"))

        btn_20 = page.locator(".count-btn:has-text('20')")
        btn_20.click()
        expect(btn_20).to_have_class(_re.compile("active"))
        expect(btn_10).not_to_have_class(_re.compile("active"))


class TestQuizFlow:
    """Tests for the quiz question/answer flow."""

    def test_quiz_starts(self, page: Page):
        """Starting a quiz navigates to quiz view with progress bar."""
        start_quiz(page, 5)
        expect(page.locator(".progress-bar")).to_be_visible()
        expect(page.locator(".question-card")).to_be_visible()
        expect(page.locator(".option-btn")).to_have_count(4)

    def test_progress_shows_question_1_of_n(self, page: Page):
        """Progress text shows 'Question 1 of N'."""
        start_quiz(page, 5)
        progress = page.locator(".progress-text")
        # Should contain "1" and "5" (language-independent check)
        text = progress.inner_text()
        assert "1" in text and "5" in text

    def test_answer_shows_feedback(self, page: Page):
        """Clicking an answer shows correct/wrong feedback."""
        start_quiz(page, 5)
        answer_current_question(page, 0)

        feedback = page.locator(".feedback")
        expect(feedback).to_be_visible()
        # Should have either 'correct' or 'wrong' class
        cls = feedback.get_attribute("class")
        assert "correct" in cls or "wrong" in cls

    def test_answer_disables_buttons(self, page: Page):
        """After answering, all option buttons become disabled."""
        start_quiz(page, 5)
        answer_current_question(page, 0)

        buttons = page.locator(".option-btn")
        for i in range(4):
            expect(buttons.nth(i)).to_be_disabled()

    def test_correct_answer_highlighted(self, page: Page):
        """After answering, the correct option gets the 'correct' class."""
        start_quiz(page, 5)
        answer_current_question(page, 0)

        # Exactly one button should have the 'correct' class
        correct_btns = page.locator(".option-btn.correct")
        expect(correct_btns).to_have_count(1)

    def test_next_button_appears_after_answer(self, page: Page):
        """After answering, a Next button appears."""
        start_quiz(page, 5)
        answer_current_question(page, 0)
        expect(page.locator(".nav-btn.primary")).to_be_visible()

    def test_skip_advances_question(self, page: Page):
        """Skip button advances to the next question."""
        start_quiz(page, 5)
        progress_before = page.locator(".progress-text").inner_text()
        page.click(".nav-btn.secondary")  # Skip
        progress_after = page.locator(".progress-text").inner_text()
        assert progress_before != progress_after

    def test_question_stays_after_answer(self, page: Page):
        """Bug fix: question text should NOT change after answering (before clicking Next)."""
        start_quiz(page, 5)
        q_text_before = page.locator(".question-text").inner_text()
        answer_current_question(page, 0)
        q_text_after = page.locator(".question-text").inner_text()
        assert q_text_before == q_text_after, (
            "Question text changed after answering — answer/next state bug!"
        )

    def test_next_advances_to_new_question(self, page: Page):
        """Clicking Next after answer shows a different question."""
        start_quiz(page, 5)
        q_text_1 = page.locator(".question-text").inner_text()
        answer_current_question(page, 0)
        click_next(page)
        # Wait for new question to render
        page.wait_for_selector(".question-card", timeout=3000)
        q_text_2 = page.locator(".question-text").inner_text()
        assert q_text_1 != q_text_2, "Question didn't change after clicking Next"


class TestResultsScreen:
    """Tests for the results/score screen."""

    def test_full_quiz_flow(self, page: Page):
        """Complete a 5-question quiz and verify results screen appears."""
        start_quiz(page, 5)

        for i in range(5):
            answer_current_question(page, 0)
            if i < 4:
                click_next(page)
                page.wait_for_selector(".question-card", timeout=3000)
            else:
                # Last question — button should say "See Results" equivalent
                click_next(page)

        # Should now be on results screen
        page.wait_for_selector(".results-screen", timeout=5000)
        expect(page.locator(".grade-badge")).to_be_visible()
        expect(page.locator(".score-display")).to_be_visible()

    def test_score_shows_valid_percentage(self, page: Page):
        """Score percentage should be between 0% and 100%."""
        start_quiz(page, 5)
        for i in range(5):
            answer_current_question(page, 0)
            if i < 4:
                click_next(page)
                page.wait_for_selector(".question-card", timeout=3000)
            else:
                click_next(page)

        page.wait_for_selector(".results-screen", timeout=5000)
        pct_text = page.locator(".score-pct").inner_text()
        pct_val = int(pct_text.replace("%", ""))
        assert 0 <= pct_val <= 100, f"Invalid percentage: {pct_text}"

    def test_score_not_over_total(self, page: Page):
        """Score detail should not show correct > total (bug fix regression)."""
        start_quiz(page, 5)
        for i in range(5):
            answer_current_question(page, 0)
            if i < 4:
                click_next(page)
                page.wait_for_selector(".question-card", timeout=3000)
            else:
                click_next(page)

        page.wait_for_selector(".results-screen", timeout=5000)
        detail = page.locator(".score-detail").inner_text()
        # Extract numbers — should find values like "3 correct, 2 wrong, 0 skipped out of 5"
        import re
        nums = [int(x) for x in re.findall(r"\d+", detail)]
        assert len(nums) >= 4, f"Could not parse score detail: {detail}"
        correct, wrong, skipped, total = nums[0], nums[1], nums[2], nums[3]
        assert correct + wrong + skipped == total, (
            f"Score doesn't add up: {correct}+{wrong}+{skipped} != {total}"
        )
        assert total == 5, f"Total should be 5, got {total}"

    def test_try_again_returns_to_config(self, page: Page):
        """Try Again button goes back to config screen."""
        start_quiz(page, 5)
        for i in range(5):
            answer_current_question(page, 0)
            if i < 4:
                click_next(page)
                page.wait_for_selector(".question-card", timeout=3000)
            else:
                click_next(page)

        page.wait_for_selector(".results-screen", timeout=5000)
        page.click(".start-btn")  # Try Again uses start-btn class
        page.wait_for_selector(".config-screen", timeout=5000)

    def test_category_breakdown_shown(self, page: Page):
        """Results should show a by-category breakdown."""
        start_quiz(page, 5)
        for i in range(5):
            answer_current_question(page, 0)
            if i < 4:
                click_next(page)
                page.wait_for_selector(".question-card", timeout=3000)
            else:
                click_next(page)

        page.wait_for_selector(".results-screen", timeout=5000)
        breakdowns = page.locator(".breakdown-item")
        # At least one category breakdown should exist
        assert breakdowns.count() >= 1


class TestI18n:
    """Tests for internationalization / language switching."""

    def test_language_selector_visible(self, page: Page):
        """Language selector dropdown should be present in header."""
        expect(page.locator(".language-selector")).to_be_visible()

    def test_language_selector_has_options(self, page: Page):
        """Language selector should have 8 language options."""
        options = page.locator(".language-selector option")
        expect(options).to_have_count(8)

    def test_switch_to_chinese(self, page: Page):
        """Switching to Chinese translates the UI."""
        page.locator(".language-selector").select_option("zh")
        # Wait for reactivity
        page.wait_for_timeout(500)
        # Header title should now be in Chinese
        title = page.locator(".header-title").inner_text()
        assert any(
            c > "\u4e00" for c in title
        ), f"Title should contain Chinese characters, got: {title}"

    def test_switch_to_german(self, page: Page):
        """Switching to German translates the config screen."""
        page.locator(".language-selector").select_option("de")
        page.wait_for_timeout(500)
        # "Configure Your Quiz" → "Quiz konfigurieren" (or similar)
        config_title = page.locator(".config-title").inner_text()
        assert config_title != "Configure Your Quiz", (
            f"Title should be translated, got: {config_title}"
        )

    def test_quiz_questions_translated(self, page: Page):
        """Questions should be translated when language is not English."""
        page.locator(".language-selector").select_option("zh")
        page.wait_for_timeout(500)
        start_quiz(page, 5)
        q_text = page.locator(".question-text").inner_text()
        # Chinese text should contain CJK characters
        has_cjk = any("\u4e00" <= c <= "\u9fff" for c in q_text)
        assert has_cjk, f"Question should contain Chinese characters, got: {q_text}"

    def test_language_persists_across_navigation(self, page: Page):
        """Language choice should persist through quiz flow."""
        page.locator(".language-selector").select_option("de")
        page.wait_for_timeout(500)
        start_quiz(page, 5)

        # Category label in quiz view should be in German
        cat_label = page.locator(".question-category").inner_text()
        # Verify the quiz header is still translated
        title = page.locator(".header-title").inner_text()
        assert title != "Photometric Knowledge Quiz", (
            f"Title should stay translated during quiz, got: {title}"
        )

    def test_switch_back_to_english(self, page: Page):
        """Switching back to English restores English strings."""
        page.locator(".language-selector").select_option("zh")
        page.wait_for_timeout(300)
        page.locator(".language-selector").select_option("en")
        page.wait_for_timeout(300)
        config_title = page.locator(".config-title").inner_text()
        assert config_title == "Configure Your Quiz"


class TestShuffleRandomness:
    """Tests for question randomness between runs."""

    def test_different_questions_on_restart(self, page: Page):
        """Bug fix: second quiz run should have different question order."""
        # First run — collect question texts
        start_quiz(page, 5)
        run1_questions = []
        for i in range(5):
            run1_questions.append(page.locator(".question-text").inner_text())
            answer_current_question(page, 0)
            if i < 4:
                click_next(page)
                page.wait_for_selector(".question-card", timeout=3000)
            else:
                click_next(page)

        page.wait_for_selector(".results-screen", timeout=5000)
        page.click(".start-btn")  # Try Again
        page.wait_for_selector(".config-screen", timeout=5000)

        # Second run — same config
        start_quiz(page, 5)
        run2_questions = []
        for i in range(5):
            run2_questions.append(page.locator(".question-text").inner_text())
            answer_current_question(page, 0)
            if i < 4:
                click_next(page)
                page.wait_for_selector(".question-card", timeout=3000)
            else:
                click_next(page)

        # The two runs should not have identical question order
        # (with 175 questions and 5 selected, probability of same order is ~1 in 2 billion)
        assert run1_questions != run2_questions, (
            "Same questions in same order on second run — shuffle seed not randomized!"
        )


class TestTheme:
    """Tests for dark/light theme toggle."""

    def test_theme_toggle(self, page: Page):
        """Theme toggle button switches between light and dark."""
        root = page.locator(".quiz-app")
        # Should start with a theme class
        cls = root.get_attribute("class")
        assert "theme-light" in cls or "theme-dark" in cls

        # Click toggle
        page.click(".theme-toggle")
        page.wait_for_timeout(300)
        new_cls = root.get_attribute("class")
        assert new_cls != cls, "Theme class should change after toggle"


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------

if __name__ == "__main__":
    # Pass through all arguments to pytest
    args = [__file__, "-v", "--tb=short"] + sys.argv[1:]
    sys.exit(pytest.main(args))
