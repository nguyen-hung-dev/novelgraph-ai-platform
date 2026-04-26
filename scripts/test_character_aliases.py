# -*- coding: utf-8 -*-
"""Test character alias extraction for one full chapter through llama.cpp.

The script is intentionally standalone and uses only Python standard library.
By default it starts a local llama.cpp server, waits until it is ready, runs the
chapter test, writes a JSON report, and stops only the server process it started.
"""

from __future__ import annotations

import argparse
import json
import subprocess
import re
import sys
import time
import urllib.error
import urllib.parse
import urllib.request
from dataclasses import dataclass, field
from datetime import datetime
from pathlib import Path
from typing import Any


DEFAULT_BASE_URL = "http://127.0.0.1:8091"
DEFAULT_MODEL = "NuExtract-1.5-smol-Q4_K_M.gguf"
DEFAULT_MODEL_PATH = "models/NuExtract-1.5-smol-Q4_K_M.gguf"
DEFAULT_SERVER_EXE = "tools/llama.cpp/llama-server.exe"
DEFAULT_CHUNK_SIZE = 2400
DEFAULT_MIN_CHUNK_SIZE = 900
DEFAULT_MAX_TOKENS = 512
DEFAULT_SERVER_CONTEXT = 4096
DEFAULT_SERVER_GPU_LAYERS = 99
DEFAULT_SERVER_START_TIMEOUT = 120


CHARACTERS_TEMPLATE = {
    "characters": [
        {
            "name": "",
            "aliases": [""],
        }
    ]
}


ALIAS_RELATIONS_TEMPLATE = {
    "character_alias_relations": [
        {
            "character": "",
            "alias": "",
        }
    ]
}


@dataclass
class TextChunk:
    index: int
    start_char: int
    end_char: int
    text: str


@dataclass
class CharacterAlias:
    name: str
    aliases: set[str] = field(default_factory=set)
    sources: list[dict[str, Any]] = field(default_factory=list)

    def to_json(self) -> dict[str, Any]:
        return {
            "name": self.name,
            "aliases": sorted(self.aliases, key=vietnamese_sort_key),
            "sources": self.sources,
        }


def main() -> int:
    configure_console_encoding()
    args = parse_args()
    input_path = Path(args.input).resolve()
    if not input_path.exists():
        print(f"Không tìm thấy file input: {input_path}", file=sys.stderr)
        return 2

    text = read_text_best_effort(input_path)
    chunks = split_text(text, args.chunk_size, args.min_chunk_size)
    if args.limit_chunks:
        chunks = chunks[: args.limit_chunks]

    output_dir = Path(args.output_dir).resolve()
    output_dir.mkdir(parents=True, exist_ok=True)

    if args.dry_run:
        report = build_base_report(args, input_path, text, chunks)
        report["dry_run"] = True
        output_path = write_report(output_dir, input_path, report)
        print(output_path)
        return 0

    server = None
    server_log_path = None
    try:
        if args.auto_server:
            server, server_log_path = ensure_llama_server(args, output_dir)

        aliases: dict[str, CharacterAlias] = {}
        chunk_reports: list[dict[str, Any]] = []

        for chunk in chunks:
            print(
                f"Chunk {chunk.index}/{len(chunks)} "
                f"chars {chunk.start_char}-{chunk.end_char}",
                flush=True,
            )
            raw_outputs: list[dict[str, Any]] = []

            if "characters" in args.passes:
                raw_outputs.append(
                    run_alias_pass(
                        args=args,
                        chunk=chunk,
                        pass_name="characters",
                        template=CHARACTERS_TEMPLATE,
                        text=text_for_nuextract(chunk.text),
                    )
                )

            if "relations" in args.passes:
                raw_outputs.append(
                    run_alias_pass(
                        args=args,
                        chunk=chunk,
                        pass_name="relations",
                        template=ALIAS_RELATIONS_TEMPLATE,
                        text=text_for_nuextract(chunk.text),
                    )
                )

            for output in raw_outputs:
                merge_alias_output(aliases, output)

            chunk_reports.append(
                {
                    "index": chunk.index,
                    "start_char": chunk.start_char,
                    "end_char": chunk.end_char,
                    "char_count": len(chunk.text),
                    "raw_outputs": raw_outputs,
                }
            )

        report = build_base_report(args, input_path, text, chunks)
        report["server"] = {
            "auto_server": args.auto_server,
            "started_by_script": server is not None,
            "server_log": str(server_log_path) if server_log_path else None,
        }
        report["characters"] = [
            item.to_json()
            for item in sorted(aliases.values(), key=lambda item: vietnamese_sort_key(item.name))
        ]
        report["chunks"] = chunk_reports
        output_path = write_report(output_dir, input_path, report)
        print(output_path)
        return 0
    finally:
        if server is not None:
            stop_llama_server(server)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Chia một chương thành chunk và test trích xuất alias nhân vật bằng NuExtract/llama.cpp.",
    )
    parser.add_argument("--input", required=True, help="Đường dẫn file text của một chương.")
    parser.add_argument("--output-dir", default="output", help="Thư mục ghi file JSON output.")
    parser.add_argument("--base-url", default=DEFAULT_BASE_URL, help="Base URL llama.cpp server.")
    parser.add_argument("--model", default=DEFAULT_MODEL, help="Model id trong llama.cpp server.")
    parser.add_argument("--model-path", default=DEFAULT_MODEL_PATH, help="Đường dẫn GGUF để tự chạy server.")
    parser.add_argument("--server-exe", default=DEFAULT_SERVER_EXE, help="Đường dẫn llama-server.exe.")
    parser.add_argument("--server-context", type=int, default=DEFAULT_SERVER_CONTEXT)
    parser.add_argument("--server-gpu-layers", type=int, default=DEFAULT_SERVER_GPU_LAYERS)
    parser.add_argument("--server-start-timeout", type=int, default=DEFAULT_SERVER_START_TIMEOUT)
    parser.add_argument(
        "--no-auto-server",
        action="store_false",
        dest="auto_server",
        help="Không tự chạy llama-server.exe; chỉ dùng server đã chạy sẵn.",
    )
    parser.set_defaults(auto_server=True)
    parser.add_argument("--chunk-size", type=int, default=DEFAULT_CHUNK_SIZE)
    parser.add_argument("--min-chunk-size", type=int, default=DEFAULT_MIN_CHUNK_SIZE)
    parser.add_argument("--max-tokens", type=int, default=DEFAULT_MAX_TOKENS)
    parser.add_argument("--timeout", type=int, default=180)
    parser.add_argument(
        "--passes",
        nargs="+",
        choices=["characters", "relations"],
        default=["characters", "relations"],
        help="Pass extraction sẽ chạy trên mỗi chunk.",
    )
    parser.add_argument("--limit-chunks", type=int, default=0, help="Chỉ chạy N chunk đầu để thử nhanh.")
    parser.add_argument("--dry-run", action="store_true", help="Chỉ chia chunk và ghi report, không gọi LLM.")
    return parser.parse_args()


def configure_console_encoding() -> None:
    for stream in (sys.stdout, sys.stderr):
        if not hasattr(stream, "reconfigure"):
            continue
        try:
            stream.reconfigure(encoding="utf-8", errors="replace")
        except Exception:
            pass


def ensure_llama_server(
    args: argparse.Namespace,
    output_dir: Path,
) -> tuple[subprocess.Popen[str] | None, Path | None]:
    if server_is_ready(args.base_url, timeout=2):
        print(f"Đang dùng llama.cpp server đã chạy sẵn: {args.base_url}", flush=True)
        return None, None

    repo_root = Path(__file__).resolve().parents[1]
    server_exe = resolve_repo_path(args.server_exe)
    model_path = resolve_repo_path(args.model_path)
    if not server_exe.exists():
        raise FileNotFoundError(f"Không tìm thấy llama-server.exe: {server_exe}")
    if not model_path.exists():
        raise FileNotFoundError(f"Không tìm thấy model GGUF: {model_path}")

    parsed = urllib.parse.urlparse(args.base_url)
    host = parsed.hostname or "127.0.0.1"
    port = parsed.port or (443 if parsed.scheme == "https" else 80)
    log_path = output_dir / f"llama-server-{datetime.now().strftime('%Y%m%d-%H%M%S')}.log"
    command = [
        str(server_exe),
        "-m",
        str(model_path),
        "--host",
        host,
        "--port",
        str(port),
        "-c",
        str(args.server_context),
        "-ngl",
        str(args.server_gpu_layers),
    ]

    print(f"Đang khởi động llama.cpp server: {args.base_url}", flush=True)
    print(f"Log server: {log_path}", flush=True)
    creationflags = getattr(subprocess, "CREATE_NO_WINDOW", 0) if sys.platform == "win32" else 0
    with log_path.open("w", encoding="utf-8", errors="replace") as log_file:
        process = subprocess.Popen(
            command,
            cwd=str(repo_root),
            stdin=subprocess.DEVNULL,
            stdout=log_file,
            stderr=subprocess.STDOUT,
            text=True,
            creationflags=creationflags,
        )

    deadline = time.time() + args.server_start_timeout
    while time.time() < deadline:
        if process.poll() is not None:
            raise RuntimeError(
                "llama.cpp server thoát trước khi sẵn sàng "
                f"(exit code {process.returncode}). Log cuối:\n{tail_file(log_path)}"
            )
        if server_is_ready(args.base_url, timeout=2):
            print("llama.cpp server đã sẵn sàng.", flush=True)
            return process, log_path
        time.sleep(0.5)

    stop_llama_server(process)
    raise TimeoutError(
        f"llama.cpp server chưa sẵn sàng sau {args.server_start_timeout}s. "
        f"Log cuối:\n{tail_file(log_path)}"
    )


def stop_llama_server(process: subprocess.Popen[str]) -> None:
    if process.poll() is not None:
        return

    print("Đang tắt llama.cpp server do script khởi động...", flush=True)
    if sys.platform == "win32":
        subprocess.run(
            ["taskkill", "/PID", str(process.pid), "/T", "/F"],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        return

    process.terminate()
    try:
        process.wait(timeout=10)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=10)


def server_is_ready(base_url: str, timeout: int) -> bool:
    try:
        get_json(f"{base_url.rstrip('/')}/v1/models", timeout)
        return True
    except Exception:
        return False


def resolve_repo_path(value: str) -> Path:
    path = Path(value)
    if path.is_absolute():
        return path
    return (Path(__file__).resolve().parents[1] / path).resolve()


def tail_file(path: Path, max_chars: int = 4000) -> str:
    if not path.exists():
        return ""
    text = path.read_text(encoding="utf-8", errors="replace")
    return text[-max_chars:]


def read_text_best_effort(path: Path) -> str:
    for encoding in ("utf-8-sig", "utf-8", "cp1258", "cp1252"):
        try:
            return path.read_text(encoding=encoding)
        except UnicodeDecodeError:
            continue
    return path.read_text(encoding="utf-8", errors="replace")


def split_text(text: str, target_size: int, min_size: int) -> list[TextChunk]:
    if target_size <= 0:
        raise ValueError("chunk-size must be greater than 0")
    if min_size < 0:
        raise ValueError("min-chunk-size must be greater than or equal to 0")

    if not text:
        return [TextChunk(index=1, start_char=0, end_char=0, text="")]

    chunks: list[TextChunk] = []
    start = 0
    text_len = len(text)
    while start < text_len:
        end = choose_chunk_end(text, start, target_size, min_size)
        chunk_text = text[start:end]
        if chunk_text.strip():
            chunks.append(
                TextChunk(
                    index=len(chunks) + 1,
                    start_char=start,
                    end_char=end,
                    text=chunk_text,
                )
            )
        start = max(end, start + 1)

    return chunks or [TextChunk(index=1, start_char=0, end_char=text_len, text=text)]


def choose_chunk_end(text: str, start: int, target_size: int, min_size: int) -> int:
    text_len = len(text)
    if text_len - start <= target_size:
        return text_len

    hard_end = min(start + target_size, text_len)
    min_end = min(start + min_size, hard_end)
    window = text[min_end:hard_end]

    for pattern in ("\n\n", "\r\n\r\n", "\n", "。", "！", "？", ".", "!", "?"):
        offset = window.rfind(pattern)
        if offset >= 0:
            return min_end + offset + len(pattern)

    return hard_end


def text_for_nuextract(chunk_text: str) -> str:
    return (
        "Chỉ tập trung trích xuất tên chính và alias/tên gọi khác/cách gọi khác "
        "của cùng nhân vật. Không trích xuất field khác.\n\n"
        f"{chunk_text}"
    )


def run_alias_pass(
    *,
    args: argparse.Namespace,
    chunk: TextChunk,
    pass_name: str,
    template: dict[str, Any],
    text: str,
) -> dict[str, Any]:
    prompt = build_nuextract_prompt(template, text)
    payload = {
        "model": args.model,
        "prompt": prompt,
        "temperature": 0,
        "max_tokens": args.max_tokens,
        "stream": False,
    }
    started = time.perf_counter()
    response = post_json(f"{args.base_url.rstrip('/')}/v1/completions", payload, args.timeout)
    elapsed_ms = round((time.perf_counter() - started) * 1000)
    content = response.get("choices", [{}])[0].get("text", "")
    parsed = parse_json_payload(content)

    return {
        "pass": pass_name,
        "chunk_index": chunk.index,
        "elapsed_ms": elapsed_ms,
        "finish_reason": response.get("choices", [{}])[0].get("finish_reason"),
        "content": content,
        "parsed": parsed,
        "usage": response.get("usage"),
        "timings": response.get("timings"),
    }


def build_nuextract_prompt(template: dict[str, Any], text: str) -> str:
    template_json = json.dumps(template, ensure_ascii=False, indent=2)
    return f"<|input|>\n### Template:\n{template_json}\n### Text:\n{text}\n\n<|output|>"


def get_json(url: str, timeout: int) -> dict[str, Any]:
    request = urllib.request.Request(
        url,
        headers={"Accept": "application/json"},
        method="GET",
    )
    with urllib.request.urlopen(request, timeout=timeout) as response:
        return json.loads(response.read().decode("utf-8"))


def post_json(url: str, payload: dict[str, Any], timeout: int) -> dict[str, Any]:
    request = urllib.request.Request(
        url,
        data=json.dumps(payload, ensure_ascii=False).encode("utf-8"),
        headers={"Content-Type": "application/json"},
        method="POST",
    )
    try:
        with urllib.request.urlopen(request, timeout=timeout) as response:
            return json.loads(response.read().decode("utf-8"))
    except urllib.error.HTTPError as error:
        body = error.read().decode("utf-8", errors="replace")
        raise RuntimeError(f"HTTP {error.code}: {body}") from error
    except urllib.error.URLError as error:
        raise RuntimeError(
            f"Không gọi được llama.cpp server tại {url}. Hãy kiểm tra server đã chạy chưa."
        ) from error


def parse_json_payload(content: str) -> Any:
    content = content.strip()
    if not content:
        return None

    for candidate in (extract_json_object(content), extract_json_array(content), content):
        if not candidate:
            continue
        try:
            return json.loads(candidate)
        except json.JSONDecodeError:
            continue
    return {"_parse_error": True, "raw": content}


def extract_json_object(content: str) -> str | None:
    start = content.find("{")
    end = content.rfind("}")
    if start < 0 or end <= start:
        return None
    return content[start : end + 1]


def extract_json_array(content: str) -> str | None:
    start = content.find("[")
    end = content.rfind("]")
    if start < 0 or end <= start:
        return None
    return content[start : end + 1]


def merge_alias_output(aliases: dict[str, CharacterAlias], output: dict[str, Any]) -> None:
    parsed = output.get("parsed")
    if not parsed or isinstance(parsed, str):
        return

    if isinstance(parsed, dict):
        for item in as_list(parsed.get("characters")):
            name = text_value(item.get("name") or item.get("primary_name") or item.get("canonical_name"))
            item_aliases = [text_value(alias) for alias in as_list(item.get("aliases"))]
            add_character_aliases(aliases, name, item_aliases, output)

        for item in as_list(parsed.get("character_alias_relations")):
            name = text_value(item.get("character") or item.get("name"))
            alias = text_value(item.get("alias"))
            add_character_aliases(aliases, name, [alias], output)

    elif isinstance(parsed, list):
        for item in parsed:
            if isinstance(item, dict):
                name = text_value(item.get("name") or item.get("character"))
                item_aliases = [text_value(alias) for alias in as_list(item.get("aliases"))]
                if item.get("alias"):
                    item_aliases.append(text_value(item.get("alias")))
                add_character_aliases(aliases, name, item_aliases, output)


def add_character_aliases(
    aliases: dict[str, CharacterAlias],
    name: str,
    item_aliases: list[str],
    output: dict[str, Any],
) -> None:
    name = clean_surface(name)
    item_aliases = [clean_surface(alias) for alias in item_aliases if clean_surface(alias)]
    if not name:
        return

    existing_key = find_existing_character_key(aliases, name, item_aliases)
    key = existing_key or surface_key(name)
    if key not in aliases:
        aliases[key] = CharacterAlias(name=name)

    for alias in item_aliases:
        if alias and surface_key(alias) != surface_key(aliases[key].name):
            aliases[key].aliases.add(alias)

    aliases[key].sources.append(
        {
            "chunk_index": output.get("chunk_index"),
            "pass": output.get("pass"),
            "elapsed_ms": output.get("elapsed_ms"),
        }
    )


def find_existing_character_key(
    aliases: dict[str, CharacterAlias],
    name: str,
    item_aliases: list[str],
) -> str | None:
    surfaces = {surface_key(name), *(surface_key(alias) for alias in item_aliases)}
    for key, item in aliases.items():
        item_surfaces = {surface_key(item.name), *(surface_key(alias) for alias in item.aliases)}
        if surfaces & item_surfaces:
            return key
    return None


def as_list(value: Any) -> list[Any]:
    if value is None:
        return []
    if isinstance(value, list):
        return value
    return [value]


def text_value(value: Any) -> str:
    if value is None:
        return ""
    return str(value)


def clean_surface(value: str) -> str:
    value = value.strip()
    value = value.strip("\"'“”‘’`")
    value = re.sub(r"\s+", " ", value)
    return value


def surface_key(value: str) -> str:
    return re.sub(r"\s+", " ", clean_surface(value).casefold())


def vietnamese_sort_key(value: str) -> str:
    return surface_key(value)


def build_base_report(
    args: argparse.Namespace,
    input_path: Path,
    text: str,
    chunks: list[TextChunk],
) -> dict[str, Any]:
    return {
        "created_at": datetime.now().astimezone().isoformat(timespec="seconds"),
        "input_file": str(input_path),
        "base_url": args.base_url,
        "model": args.model,
        "passes": args.passes,
        "chunk_size": args.chunk_size,
        "min_chunk_size": args.min_chunk_size,
        "chapter_char_count": len(text),
        "chunk_count": len(chunks),
    }


def write_report(output_dir: Path, input_path: Path, report: dict[str, Any]) -> Path:
    timestamp = datetime.now().strftime("%Y%m%d-%H%M%S")
    safe_stem = re.sub(r"[^A-Za-z0-9_.-]+", "-", input_path.stem).strip("-") or "chapter"
    output_path = output_dir / f"character-aliases-{safe_stem}-{timestamp}.json"
    output_path.write_text(
        json.dumps(report, ensure_ascii=False, indent=2),
        encoding="utf-8",
    )
    return output_path


if __name__ == "__main__":
    raise SystemExit(main())
