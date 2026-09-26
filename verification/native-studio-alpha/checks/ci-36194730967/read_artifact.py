import datetime
import hashlib
import io
import json
import pathlib
import subprocess
import urllib.error
import urllib.request
import zipfile

ROOT = pathlib.Path(__file__).resolve().parent
API = "https://api.github.com/repos/agentique-systems/agentique/actions/artifacts/10890283026"
token = subprocess.check_output(["gh", "auth", "token"], text=True).strip()
headers = {"Authorization": "Bearer " + token, "Accept": "application/vnd.github+json", "X-GitHub-Api-Version": "2022-11-28"}
metadata = json.load(urllib.request.urlopen(urllib.request.Request(API, headers=headers)))
assert metadata["id"] == 10890283026
assert metadata["workflow_run"]["head_sha"] == "4747f8eee085c939932d7be00abbf6af5a1c0ad2"
assert metadata["size_in_bytes"] == 183970169

class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, response_headers, newurl):
        return None

try:
    urllib.request.build_opener(NoRedirect).open(urllib.request.Request(API + "/zip", headers=headers))
    raise RuntimeError("Expected authenticated artifact redirect")
except urllib.error.HTTPError as response:
    assert response.code == 302
    download_url = response.headers["Location"]

class RemoteZip(io.RawIOBase):
    def __init__(self):
        self.position = 0
        self.size = metadata["size_in_bytes"]
        self.transferred = 0
        self.requests = []
        self.etag = None

    def seekable(self):
        return True

    def readable(self):
        return True

    def tell(self):
        return self.position

    def seek(self, offset, whence=0):
        self.position = offset if whence == 0 else self.position + offset if whence == 1 else self.size + offset
        assert 0 <= self.position <= self.size
        return self.position

    def read(self, size=-1):
        size = self.size - self.position if size < 0 else min(size, self.size - self.position)
        if size == 0:
            return b""
        if size > 4 * 1024 * 1024:
            raise RuntimeError("Refusing heavy artifact transfer during semantic oracle")
        start, end = self.position, self.position + size - 1
        request_headers = {"Range": f"bytes={start}-{end}"}
        if self.etag:
            request_headers["If-Match"] = self.etag
        with urllib.request.urlopen(urllib.request.Request(download_url, headers=request_headers), timeout=30) as response:
            assert response.status == 206, response.status
            assert response.headers["Content-Range"] == f"bytes {start}-{end}/{self.size}"
            current_etag = response.headers["ETag"]
            assert self.etag is None or current_etag == self.etag
            self.etag = current_etag
            data = response.read(size + 1)
            assert len(data) == size
        self.position += size
        self.transferred += size
        self.requests.append({"offset": start, "bytes": size})
        return data

started = datetime.datetime.now(datetime.timezone.utc).isoformat()
stream = RemoteZip()
extracted = []
with zipfile.ZipFile(stream) as archive:
    names = archive.namelist()
    (ROOT / "archive-names.txt").write_text("\n".join(names) + "\n", encoding="utf-8")
    wanted = [
        "verification/generated/logs/rust-clippy.txt",
        "verification/generated/logs/engineering-tests.txt",
        "verification/generated/logs/independent-fixtures.txt",
        "verification/generated/logs/standards.txt",
        "verification/independent-fixtures.json",
        "verification/results.json",
    ]
    for suffix in wanted:
        matches = [name for name in names if name == suffix or name.endswith("/" + suffix)]
        if len(matches) != 1:
            raise RuntimeError(f"Expected one {suffix}: {matches}")
        member = matches[0]
        info = archive.getinfo(member)
        assert info.file_size < 4 * 1024 * 1024
        data = archive.read(member)
        destination = ROOT / pathlib.PurePosixPath(member).name
        destination.write_bytes(data)
        extracted.append({"member": member, "local_name": destination.name, "bytes": len(data), "sha256": hashlib.sha256(data).hexdigest(), "zip_crc32": f"{info.CRC:08x}"})

record = {"format": "agentique-ci-selective-artifact-read/1", "started_at": started, "completed_at": datetime.datetime.now(datetime.timezone.utc).isoformat(), "artifact": metadata, "requests": stream.requests, "transferred_bytes": stream.transferred, "etag": stream.etag, "outer_archive_digest_verified": False, "qualification": "Authenticated exact artifact API and stable ETag range reads; ZIP entry CRC and retained entry SHA256 verified. Full archive SHA256 remains unverified without full transfer.", "extracted": extracted}
(ROOT / "acquisition.json").write_text(json.dumps(record, indent=2) + "\n", encoding="utf-8")
print(json.dumps({"transferred_bytes": stream.transferred, "extracted": extracted, "completed_at": record["completed_at"]}, indent=2))
