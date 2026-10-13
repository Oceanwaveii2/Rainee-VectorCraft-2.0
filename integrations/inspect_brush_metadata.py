"""Read-only local user-file inventory. Never execute or emit embedded PostScript.

Default mode reports metadata only. Explicit --local-shapes exports conservative local
SVG silhouettes from user artwork, never proprietary programs, presets or training data.
PDF stream decompression is explicitly bounded before private metadata inspection.
"""
import hashlib
import io
import json
import re
import sys
import zlib
import html
import time
from pathlib import Path

from pypdf import PdfReader

MAX_FILE = 512 * 1024 * 1024
MAX_PRIVATE = 256 * 1024 * 1024


def stream_inventory(decoder):
    """Count metadata markers to EOF without keeping or executing embedded programs.

    Bound decoding to 8 GiB / 120 seconds; explicitly report a limit instead of implying
    exhaustive coverage. Keep at most a 1 MiB chunk and 8 KiB boundary overlap.
    """
    marker = re.compile(rb"%AI\d+_(BeginBrushPattern|EndBrushPattern|BeginPattern|BeginLayer)\b")
    counts = {key: 0 for key in ("BeginBrushPattern", "EndBrushPattern", "BeginPattern", "BeginLayer")}
    labels, carry, total, complete = [], b"", 0, False
    start = time.monotonic()
    while total <= 8 * 1024**3 and time.monotonic() - start < 120:
        chunk = decoder.read(1024 * 1024)
        total += len(chunk)
        buffer = carry + chunk
        boundary = len(buffer) if not chunk else max(0, len(buffer) - 8192)
        for match in marker.finditer(buffer):
            if match.start() >= boundary:
                break
            key = match.group(1).decode()
            counts[key] += 1
            if key == "BeginBrushPattern":
                candidates = re.findall(rb"\(([^()\r\n]{1,100})\)", buffer[match.end():match.end()+1024])
                labels.append([value.decode("utf-8", errors="replace") for value in candidates[:8]])
        carry = buffer[boundary:]
        if not chunk:
            complete = True
            break
    return {"decodedBytes": total, "completeToEOF": complete, "sections": counts,
            "brushStringCandidates": labels, "balancedBrushSections": counts["BeginBrushPattern"] == counts["EndBrushPattern"],
            "limit": None if complete else "8 GiB or 120 second decoding limit",
            "scope": "marker inventory, not original color/dynamic brush fidelity"}


def shape_svg(section, output, index, label):
    """Conservative path-only reader of user artwork, never an executable interpreter.

    Keeps compound holes. Named-color/appearance metadata is deliberately not interpreted:
    output is a monochrome silhouette candidate, not original dynamic settings.
    """
    paths, current, points, compound = [], [], [], []
    depth = 0
    for line in section.splitlines():
        tokens = line.split()
        if not tokens or line.lstrip().startswith(b"%"):
            continue
        op = tokens[-1]
        if op == b"*u":
            depth += 1
            continue
        if op == b"*U":
            depth = max(0, depth - 1)
            if not depth and compound:
                paths.append(" ".join(compound)); compound = []
            continue
        if op in (b"m", b"l", b"L", b"c", b"C", b"v", b"V", b"y", b"Y"):
            try:
                values = [float(value) for value in tokens[:-1]]
            except ValueError:
                continue
            import math
            if not all(math.isfinite(value) and abs(value) <= 1e6 for value in values):
                raise ValueError("Invalid brush path coordinates")
            if op == b"m" and len(values) == 2:
                current = [f"M{values[0]} {-values[1]}"]
            elif op in (b"l", b"L") and len(values) == 2:
                current.append(f"L{values[0]} {-values[1]}")
            elif op in (b"c", b"C") and len(values) == 6:
                current.append("C" + " ".join(str(value if i % 2 == 0 else -value) for i, value in enumerate(values)))
            elif op in (b"y", b"Y") and len(values) == 4:
                current.append(f"C{values[0]} {-values[1]} {values[2]} {-values[3]} {values[2]} {-values[3]}")
            elif op in (b"v", b"V"):
                raise ValueError("Short cubic with implicit initial control unsupported")
            points.extend(zip(values[::2], [-value for value in values[1::2]]))
        elif op in (b"f", b"F", b"b", b"B") and current:
            if op in (b"f", b"b"):
                current.append("Z")
            (compound if depth else paths).append(" ".join(current)); current = []
        elif op in (b"n", b"N"):
            current = []
    if compound:
        paths.append(" ".join(compound))
    if not paths or not points:
        raise ValueError("No usable filled brush paths")
    xs, ys = zip(*points)
    x0, y0, x1, y1 = min(xs), min(ys), max(xs), max(ys)
    if x1 <= x0 or y1 <= y0:
        raise ValueError("Empty brush envelope")
    # Bounds include Bezier controls and non-painted source envelope paths, conservatively.
    content = f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{x0} {y0} {x1-x0} {y1-y0}"><title>{html.escape(label)}</title>'
    content += ''.join(f'<path fill="black" fill-rule="evenodd" d="{path}"/>' for path in paths) + '</svg>'
    destination = output / f"brush-{index:02d}.svg"
    destination.write_text(content, encoding="utf-8")
    return {"svg": str(destination), "filledPaths": len(paths), "bounds": [x0, y0, x1, y1], "fidelity": "monochrome geometry candidate; colors and dynamics not recovered"}


def inspect(path, output=None, full_inventory=False):
    path = Path(path)
    if path.stat().st_size > MAX_FILE:
        raise ValueError("Source exceeds local inventory size limit")
    digest = hashlib.sha256()
    with path.open("rb") as source:
        for chunk in iter(lambda: source.read(1024 * 1024), b""):
            digest.update(chunk)
    reader = PdfReader(path, strict=False)
    result = {"source": str(path), "sha256": digest.hexdigest(), "pages": len(reader.pages)}
    if not reader.pages:
        return result
    private = reader.pages[0].get("/PieceInfo", {}).get("/Illustrator", {})
    private = private.get_object().get("/Private", {}) if private else {}
    private = private.get_object() if private else {}
    blocks = []
    size = 0
    for index in range(1, 10001):
        entry = private.get(f"/AIPrivateData{index}")
        if entry is None:
            break
        stream = entry.get_object()
        raw = stream._data
        filters = stream.get("/Filter")
        if filters in ("/FlateDecode", ["/FlateDecode"]):
            decoder = zlib.decompressobj()
            raw = decoder.decompress(raw, MAX_PRIVATE - size + 1)
            if not decoder.eof:
                raise ValueError("Private stream exceeds limit or is incomplete")
        elif filters:
            raise ValueError("Unsupported private stream filter")
        size += len(raw)
        if size > MAX_PRIVATE:
            raise ValueError("Private data exceeds inventory limit")
        blocks.append(raw)
    data = b"".join(blocks)
    result["privateBytes"] = len(data)
    marker = b"%AI12_CompressedData"
    if marker in data:
        header, body = data.split(marker, 1)
        decoder = zlib.decompressobj()
        decoded = decoder.decompress(body.lstrip(b"\r\n"), MAX_PRIVATE + 1)
        if len(decoded) > MAX_PRIVATE or not decoder.eof:
            raise ValueError("Inner private data exceeds limit or is incomplete")
        data = header + decoded
    if b"%AI24_ZStandard_Data" in data:
        sys.path.insert(0, "D:/DevData/Rainee/research/brush-audit-deps")
        import zstandard
        header, body = data.split(b"%AI24_ZStandard_Data", 1)
        body = body.rstrip(b"\0")
        with zstandard.ZstdDecompressor(max_window_size=64 << 20).stream_reader(io.BytesIO(body), read_across_frames=True) as decoder:
            if full_inventory:
                result["fullInventory"] = stream_inventory(decoder)
                for key in result["fullInventory"]["sections"]:
                    result["fullInventory"]["sections"][key] += len(re.findall(rb"%AI\d+_" + key.encode() + rb"\b", header))
                sections = result["fullInventory"]["sections"]
                result["fullInventory"]["balancedBrushSections"] = sections["BeginBrushPattern"] == sections["EndBrushPattern"]
                return result
            decoded = decoder.read(MAX_PRIVATE + 1)
        result["partial"] = len(decoded) > MAX_PRIVATE
        data = header + decoded[:MAX_PRIVATE]
    result["decodedBytes"] = len(data)
    result["sections"] = {
        name: len(re.findall(rb"%AI\d+_" + name.encode() + rb"\b", data))
        for name in ("BeginBrushPattern", "EndBrushPattern", "BeginPattern", "BeginLayer")
    }
    result["brushStringCandidates"] = []
    result["brushGeometry"] = []
    for index, match in enumerate(re.finditer(rb"%AI\d+_BeginBrushPattern\b", data), 1):
        # Candidate labels only, no vector programs or image payloads leave this tool.
        block = data[match.end():match.end() + 1024]
        candidates = re.findall(rb"\(([^()\r\n]{1,100})\)", block)
        result["brushStringCandidates"].append([value.decode("utf-8", errors="replace") for value in candidates[:8]])
        ending = re.search(rb"%AI\d+_EndBrushPattern\b", data[match.end():])
        section = data[match.end():match.end() + ending.start()] if ending else b""
        operators = re.findall(rb"(?m)([A-Za-z*]+)\s*$", section)
        counts = {op.decode(): operators.count(op) for op in set(operators)}
        geometry = {"bytes": len(section), "operators": counts}
        if output:
            output.mkdir(parents=True, exist_ok=True)
            try:
                label = candidates[0].decode("utf-8", errors="replace") if candidates else f"Brush {index}"
                geometry.update(shape_svg(section, output, index, label))
            except ValueError as error:
                geometry["error"] = str(error)
        result["brushGeometry"].append(geometry)
    result["status"] = "Section inventory only; no brush names or dynamics inferred"
    return result


if __name__ == "__main__":
    arguments = sys.argv[1:]
    output = None
    full_inventory = arguments[:1] == ["--full-inventory"]
    if full_inventory:
        arguments = arguments[1:]
    if arguments[:1] == ["--local-shapes"]:
        output = Path(arguments[1]); arguments = arguments[2:]
    for filename in arguments:
        try:
            print(json.dumps(inspect(filename, output, full_inventory)))
        except Exception as error:
            print(json.dumps({"source": filename, "error": str(error)}))
