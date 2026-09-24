"""Check actual Studio HTTP startup without granting semantic acceptance.

An explicitly missing accepted cache must produce an honest setup error while
the built operator application remains available. This is not the real-model gate.
"""
from pathlib import Path
import json
import os
import subprocess
import time
import urllib.error
import urllib.request

ROOT = Path(__file__).resolve().parents[2]


def main():
    binary = ROOT / "target/debug" / ("agq-studio.exe" if os.name == "nt" else "agq-studio")
    output = ROOT / "verification/generated/agentique-studio-phase3"
    output.mkdir(parents=True, exist_ok=True)
    command = [str(binary), "--port", "7346", "--database", str(output / "startup-smoke.sqlite"),
               "--kerml-cache", str(output / "deliberately-missing-accepted-cache.zip")]
    assert not Path(command[-1]).exists(), "negative-fixture path unexpectedly exists"
    with (output / "startup-process.log").open("w", encoding="utf-8") as stream:
        process = subprocess.Popen(command, cwd=ROOT, stdout=stream, stderr=subprocess.STDOUT,
                                   creationflags=subprocess.CREATE_NO_WINDOW if os.name == "nt" else 0)
        try:
            start = time.monotonic()
            while True:
                try:
                    with urllib.request.urlopen("http://127.0.0.1:7346/studio", timeout=2) as response:
                        html = response.read().decode()
                        assert response.status == 200 and 'id="root"' in html
                        break
                except urllib.error.URLError:
                    assert process.poll() is None, "host exited before readiness"
                    if time.monotonic() - start > 20:
                        raise TimeoutError("Studio host did not serve the built application")
                    time.sleep(0.1)
            try:
                urllib.request.urlopen("http://127.0.0.1:7346/api/gen2/studio/session", timeout=5)
                raise AssertionError("missing cache was presented as a ready semantic session")
            except urllib.error.HTTPError as error:
                assert error.code == 503, error.code
                body = json.loads(error.read())
                assert "AGENTIQUE_KERML_CACHE" in body["description"], body
                print(json.dumps({"application_http": 200, "session_http": 503,
                                  "description": body["description"],
                                  "semantic_acceptance": False, "command": command}))
        finally:
            process.terminate()
            process.wait(timeout=10)


if __name__ == "__main__":
    main()
