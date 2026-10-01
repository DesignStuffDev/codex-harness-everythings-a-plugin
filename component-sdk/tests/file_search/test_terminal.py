"""Validate the observation helper only; never count these as product tests."""

import unittest
from terminal import Screen


class TerminalObservationTests(unittest.TestCase):
    def test_split_color_highlight_and_cursor_edits_reconstruct_current_filename(self):
        screen = Screen(4, 60)
        stream = b"\x1b[2;4Hp02\x1b[1;35malpha\x1b[0m_visible.rs"
        for byte in stream:
            screen.feed(bytes([byte]))
        self.assertIn("p02alpha_visible.rs", screen.text())
        screen.feed(b"\x1b[2;1H\x1b[2K\x1b[3;1HSearching...")
        self.assertNotIn("p02alpha_visible.rs", screen.text())
        self.assertIn("Searching...", screen.text())
        self.assertEqual(screen.unknown, set())

    def test_queries_reply_across_every_byte_split_without_leaking_into_screen(self):
        stream = b"\x1b[6n\x1b[c\x1b[?u\x1b]10;?\x1b\\\x1b]11;?\a"
        expected = b"\x1b[1;1R\x1b[?1;2c\x1b[?0u\x1b]10;rgb:ffff/ffff/ffff\x1b\\\x1b]11;rgb:0000/0000/0000\x1b\\"
        for split in range(len(stream) + 1):
            screen = Screen()
            self.assertEqual(
                screen.feed(stream[:split]) + screen.feed(stream[split:]), expected
            )
            self.assertFalse(screen.text().strip())
            self.assertEqual(screen.unknown, set())

    def test_scroll_and_clear_do_not_leave_stale_match_on_visible_screen(self):
        screen = Screen(3, 40)
        screen.feed(b"stale_match.rs\r\nsecond\r\nthird\r\nlast")
        self.assertNotIn("stale_match.rs", screen.text())
        self.assertIn("last", screen.text())
        screen.feed(b"\x1b[2J\x1b[1;1Hfresh_match.rs")
        self.assertEqual(screen.text().strip(), "fresh_match.rs")

    def test_erase_scrollback_preserves_visible_filename(self):
        screen = Screen()
        screen.feed(b"visible_match.rs\x1b[3J")
        self.assertIn("visible_match.rs", screen.text())

    def test_unknown_cursor_mutation_is_not_silently_accepted(self):
        screen = Screen()
        screen.feed(b"\x1b[?123z")
        self.assertTrue(screen.unknown)


if __name__ == "__main__":
    unittest.main()
