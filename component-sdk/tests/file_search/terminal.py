"""Small VT screen observer for fixed-size ASCII filename acceptance, not a TUI.

Parses the cursor/erase/scroll operations emitted by Ratatui. Unsupported screen
mutations are retained and make acceptance fail rather than silently inventing a
screen. SGR, terminal mode negotiation, OSC color/title and graphics are skipped.
"""

import codecs
import unicodedata


class Screen:
    def __init__(self, rows=40, cols=140):
        self.rows, self.cols = rows, cols
        self.lines = [[" "] * cols for _ in range(rows)]
        self.row = self.col = 0
        self.saved = (0, 0)
        self.top, self.bottom = 0, rows - 1
        self.state, self.sequence = "text", ""
        self.decoder = codecs.getincrementaldecoder("utf-8")("replace")
        self.unknown = set()
        self.replies = bytearray()

    def text(self):
        return "\n".join("".join(line).rstrip() for line in self.lines)

    def feed(self, data):
        self.replies.clear()
        for char in self.decoder.decode(data):
            self._char(char)
        return bytes(self.replies)

    def _linefeed(self):
        if self.row == self.bottom:
            self.lines.pop(self.top)
            self.lines.insert(self.bottom, [" "] * self.cols)
        else:
            self.row = min(self.rows - 1, self.row + 1)

    def _char(self, char):
        if self.state == "osc":
            if char == "\a":
                self.state = "text"
            elif char == "\x1b":
                self.state = "osc-escape"
            else:
                self.sequence = (self.sequence + char)[-1024:]
            if self.state == "text":
                self._osc()
            return
        if self.state == "osc-escape":
            if char == "\\":
                self._osc()
                self.state = "text"
            else:
                self.state = "osc"
            return
        if self.state == "charset":
            self.state = "text"
            return
        if self.state == "escape":
            self.state = "text"
            if char == "[":
                self.state, self.sequence = "csi", ""
            elif char == "]":
                self.state, self.sequence = "osc", ""
            elif char in "()#":
                self.state = "charset"
            elif char == "7":
                self.saved = self.row, self.col
            elif char == "8":
                self.row, self.col = self.saved
            elif char == "D":
                self._linefeed()
            elif char == "E":
                self.col = 0
                self._linefeed()
            elif char == "M":
                if self.row == self.top:
                    self.lines.pop(self.bottom)
                    self.lines.insert(self.top, [" "] * self.cols)
                else:
                    self.row = max(0, self.row - 1)
            elif char not in "=>":
                self.unknown.add("ESC " + repr(char))
            return
        if self.state == "csi":
            if "@" <= char <= "~":
                self._csi(char, self.sequence)
                self.state = "text"
            elif len(self.sequence) < 128:
                self.sequence += char
            else:
                self.unknown.add("oversized CSI")
                self.state = "text"
            return
        if char == "\x1b":
            self.state = "escape"
        elif char == "\r":
            self.col = 0
        elif char in "\n\v\f":
            self._linefeed()
        elif char == "\b":
            self.col = max(0, self.col - 1)
        elif char == "\t":
            self.col = min(self.cols - 1, (self.col // 8 + 1) * 8)
        elif char >= " " and char != "\x7f":
            width = (
                0
                if unicodedata.combining(char)
                else 2
                if unicodedata.east_asian_width(char) in "WF"
                else 1
            )
            if width == 0:
                return  # assertions use ASCII fixture filenames only
            if self.col >= self.cols:
                self.col = 0
                self._linefeed()
            self.lines[self.row][self.col] = char
            if width == 2 and self.col + 1 < self.cols:
                self.lines[self.row][self.col + 1] = ""
            self.col += width

    def _osc(self):
        if self.sequence in ("10;?", "11;?"):
            color = (
                "ffff/ffff/ffff" if self.sequence.startswith("10") else "0000/0000/0000"
            )
            self.replies.extend(f"\x1b]{self.sequence[:2]};rgb:{color}\x1b\\".encode())

    def _csi(self, final, body):
        if final == "n" and body == "6":
            self.replies.extend(
                f"\x1b[{self.row + 1};{min(self.col, self.cols - 1) + 1}R".encode()
            )
            return
        if final == "c":
            self.replies.extend(b"\x1b[?1;2c")
            return
        if final == "u" and body == "?":
            self.replies.extend(b"\x1b[?0u")
            return
        if final in "mhlq" or (final == "u" and body.startswith((">", "<", "="))):
            return
        try:
            args = [int(x) if x else 0 for x in body.split(";")] if body else [0]
        except ValueError:
            self.unknown.add("CSI " + body + final)
            return
        n = args[0] or 1
        if final in "Hf":
            self.row = min(self.rows - 1, max(0, n - 1))
            self.col = min(
                self.cols - 1, max(0, (args[1] if len(args) > 1 and args[1] else 1) - 1)
            )
        elif final == "A":
            self.row = max(self.top, self.row - n)
        elif final in "Be":
            self.row = min(self.bottom, self.row + n)
        elif final in "Ca":
            self.col = min(self.cols - 1, self.col + n)
        elif final == "D":
            self.col = max(0, self.col - n)
        elif final == "E":
            self.row = min(self.bottom, self.row + n)
            self.col = 0
        elif final == "F":
            self.row = max(self.top, self.row - n)
            self.col = 0
        elif final in "G`":
            self.col = min(self.cols - 1, n - 1)
        elif final == "d":
            self.row = min(self.rows - 1, n - 1)
        elif final == "K":
            lo, hi = (
                (0, self.cols)
                if args[0] == 2
                else (0, self.col + 1)
                if args[0] == 1
                else (self.col, self.cols)
            )
            self.lines[self.row][lo:hi] = [" "] * (hi - lo)
        elif final == "J":
            if args[0] == 2:
                self.lines = [[" "] * self.cols for _ in range(self.rows)]
            elif args[0] == 3:
                pass  # ED3 clears scrollback, never visible cells
            elif args[0] == 0:
                self.lines[self.row][self.col :] = [" "] * (self.cols - self.col)
                for row in range(self.row + 1, self.rows):
                    self.lines[row] = [" "] * self.cols
            elif args[0] == 1:
                for row in range(self.row):
                    self.lines[row] = [" "] * self.cols
                self.lines[self.row][: self.col + 1] = [" "] * (self.col + 1)
        elif final in "ST":
            for _ in range(min(n, self.bottom - self.top + 1)):
                if final == "S":
                    self.lines.pop(self.top)
                    self.lines.insert(self.bottom, [" "] * self.cols)
                else:
                    self.lines.pop(self.bottom)
                    self.lines.insert(self.top, [" "] * self.cols)
        elif final in "LM":
            for _ in range(min(n, self.bottom - self.row + 1)):
                if final == "L":
                    self.lines.pop(self.bottom)
                    self.lines.insert(self.row, [" "] * self.cols)
                else:
                    self.lines.pop(self.row)
                    self.lines.insert(self.bottom, [" "] * self.cols)
        elif final == "X":
            hi = min(self.cols, self.col + n)
            self.lines[self.row][self.col : hi] = [" "] * (hi - self.col)
        elif final == "P":
            n = min(n, self.cols - self.col)
            self.lines[self.row][self.col :] = (
                self.lines[self.row][self.col + n :] + [" "] * n
            )
        elif final == "@":
            n = min(n, self.cols - self.col)
            self.lines[self.row][self.col :] = [" "] * n + self.lines[self.row][
                self.col : self.cols - n
            ]
        elif final == "r":
            self.top = n - 1
            self.bottom = (args[1] if len(args) > 1 and args[1] else self.rows) - 1
        elif final == "s":
            self.saved = self.row, self.col
        elif final == "u":
            self.row, self.col = self.saved
        elif final == "t":
            pass  # terminal geometry/window query; no screen mutation
        else:
            self.unknown.add("CSI " + body + final)
