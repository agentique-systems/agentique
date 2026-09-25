"""Inspect retained Actions ZIP directories using authenticated bounded range reads.

Authentication tokens and signed download URLs stay in memory and are never logged.
No archive members are extracted or treated as accepted publication authority.
"""
import concurrent.futures
import io
import json
import pathlib
import subprocess
import urllib.error
import urllib.request
import zipfile

HERE = pathlib.Path(__file__).resolve().parent
listing = json.loads(json.loads((HERE / "search-1.json").read_text())["output"])
token = subprocess.check_output(["gh", "auth", "token"], text=True).strip()

class NoRedirect(urllib.request.HTTPRedirectHandler):
    def redirect_request(self, req, fp, code, msg, headers, newurl):
        return None

class RemoteZip(io.RawIOBase):
    def __init__(self, url):
        self.url = url
        self.offset = 0
        self.downloaded = 0
        with urllib.request.urlopen(urllib.request.Request(url, method="HEAD"), timeout=60) as response:
            self.size = int(response.headers["Content-Length"])

    def seekable(self):
        return True

    def seek(self, offset, whence=0):
        self.offset = offset + (0 if whence == 0 else self.offset if whence == 1 else self.size)
        return self.offset

    def tell(self):
        return self.offset

    def read(self, count=-1):
        count = min(self.size - self.offset, self.size if count < 0 else count)
        if count <= 0:
            return b""
        if count > 64 * 1024 * 1024:
            raise ValueError("Central-directory read exceeds bound")
        request = urllib.request.Request(self.url, headers={"Range": f"bytes={self.offset}-{self.offset+count-1}"})
        with urllib.request.urlopen(request, timeout=60) as response:
            if response.status != 206:
                raise ValueError("Storage did not honor bounded range read")
            data = response.read(count + 1)
        if len(data) != count:
            raise ValueError("Truncated range response")
        self.offset += count
        self.downloaded += count
        return data

def inspect(artifact):
    record = {"id": artifact["id"], "name": artifact["name"], "expired": artifact["expired"]}
    if artifact["expired"]:
        record["status"] = "expired"
        return record
    try:
        request = urllib.request.Request(artifact["archive_download_url"], headers={
            "Authorization": "Bearer " + token, "Accept": "application/vnd.github+json"})
        try:
            urllib.request.build_opener(NoRedirect).open(request, timeout=60)
            raise ValueError("Expected isolated signed storage redirect")
        except urllib.error.HTTPError as redirect:
            if redirect.code != 302:
                raise
            url = redirect.headers["Location"]
        source = RemoteZip(url)
        with zipfile.ZipFile(source) as archive:
            entries = archive.infolist()
            candidates = [entry for entry in entries if any(part in entry.filename.lower()
                for part in ("publication.zip", ".agq-runtime", "kerml.cache", "systems.cache", "kernel.jsonl", "closure.json"))
                or entry.file_size == 540739848]
            record.update(status="inspected", files=len(entries), directory_bytes_read=source.downloaded,
                          archive_bytes=source.size, candidates=[{"name": e.filename, "bytes": e.file_size} for e in candidates])
    except Exception as error:
        # Error messages may contain signed URLs; retain only safe classifications.
        record.update(status="failed", error_type=type(error).__name__, http_status=getattr(error, "code", None))
    return record

with concurrent.futures.ThreadPoolExecutor(max_workers=3) as pool:
    records = list(pool.map(inspect, listing["artifacts"]))
result = {"artifacts": records, "inspected": sum(r["status"] == "inspected" for r in records),
          "candidate_entries": sum(len(r.get("candidates", [])) for r in records),
          "failed": sum(r["status"] == "failed" for r in records)}
(HERE / "actions-archive-inventory.json").write_text(json.dumps(result, indent=2), encoding="utf-8")
print(json.dumps({key: value for key, value in result.items() if key != "artifacts"}), flush=True)
