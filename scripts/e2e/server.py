"""Local fixture server for the end-to-end run: serves a directory with Range support.

`http.server` ignores Range headers, which would silently turn the segmented and resume
cases into plain downloads, so this handler answers 206 itself.
"""
import re
import sys
from functools import partial
from http.server import HTTPServer, SimpleHTTPRequestHandler


class RangeHandler(SimpleHTTPRequestHandler):
    def log_message(self, *_args):
        pass

    def send_head(self):
        header = self.headers.get("Range")
        path = self.translate_path(self.path)
        match = re.fullmatch(r"bytes=(\d+)-(\d*)", header or "")
        try:
            handle = open(path, "rb")
        except OSError:
            return super().send_head()
        if not match:
            return super().send_head()
        handle.seek(0, 2)
        size = handle.tell()
        start = int(match.group(1))
        end = int(match.group(2)) if match.group(2) else size - 1
        if start >= size:
            self.send_response(416)
            self.send_header("Content-Range", f"bytes */{size}")
            self.end_headers()
            return None
        end = min(end, size - 1)
        handle.seek(start)
        self.send_response(206)
        self.send_header("Content-Range", f"bytes {start}-{end}/{size}")
        self.send_header("Content-Length", str(end - start + 1))
        self.send_header("Accept-Ranges", "bytes")
        self.end_headers()
        self._remaining = end - start + 1
        return handle

    def copyfile(self, source, outputfile):
        remaining = getattr(self, "_remaining", None)
        if remaining is None:
            return super().copyfile(source, outputfile)
        outputfile.write(source.read(remaining))


if __name__ == "__main__":
    root, port = sys.argv[1], int(sys.argv[2])
    HTTPServer(("127.0.0.1", port), partial(RangeHandler, directory=root)).serve_forever()
