"""Read logical evidence bytes, including losslessly archived large exports."""
import gzip
import json
from pathlib import Path

def read_json(path):
    path=Path(path)
    data=path.read_bytes() if path.exists() else gzip.decompress(path.with_suffix(path.suffix+'.gz').read_bytes())
    return json.loads(data)
