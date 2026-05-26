"""HTTP cache shared by webref + tc39 biblio paths.

Conditional GET (ETag / Last-Modified) against ~/.cache/elidex-webref/ (or
$XDG_CACHE_HOME/elidex-webref/). Cache is an optimization, not a correctness
layer — any I/O failure falls back to uncached fetch.
"""
from __future__ import annotations

import hashlib
import json
import os
import sys
import urllib.error
import urllib.request
from pathlib import Path


class NotFound(Exception):
    """Raised by cached_fetch_url() on HTTP 404 so callers can attempt a fallback."""


def _cache_dir() -> Path | None:
    """Return the cache directory, or None if it can't be created.

    Returning None (rather than raising) lets `cached_fetch_url` treat
    cache I/O failures (read-only home, unwritable XDG_CACHE_HOME, full
    disk) as a degradation to uncached fetch — cache is an optimization,
    not a correctness layer.
    """
    base = os.environ.get("XDG_CACHE_HOME")
    root = Path(base) if base else Path.home() / ".cache"
    d = root / "elidex-webref"
    try:
        d.mkdir(parents=True, exist_ok=True)
    except OSError:
        return None
    return d


def cached_fetch_url(url: str) -> bytes:
    """GET `url` with conditional-GET caching.

    Stores the response body alongside ETag / Last-Modified in
    ~/.cache/elidex-webref/ (or $XDG_CACHE_HOME/elidex-webref/ when set).
    On subsequent calls sends If-None-Match / If-Modified-Since; on 304
    returns the cached body. Set ELIDEX_WEBREF_NO_CACHE=1 to bypass
    entirely (uncached fetch). If the cache directory or any cache file
    is unwritable, transparently falls back to uncached fetch — cache is
    an optimization, not a correctness layer.

    Raises NotFound on 404, exits on other network / HTTP errors.
    """
    if os.environ.get("ELIDEX_WEBREF_NO_CACHE"):
        return _raw_fetch(url, headers={})

    cdir = _cache_dir()
    if cdir is None:
        return _raw_fetch(url, headers={})

    key = hashlib.sha1(url.encode("utf-8")).hexdigest()
    body_path = cdir / key
    meta_path = cdir / f"{key}.meta"

    headers: dict[str, str] = {}
    if body_path.exists() and meta_path.exists():
        try:
            meta = json.loads(meta_path.read_text("utf-8"))
        except (OSError, json.JSONDecodeError):
            meta = {}
        # Trust-boundary defense — `.meta` is on-disk user-mutable input.
        # `json.loads` can return any JSON value (`[]`, `"..."`, `null`); a
        # corrupted or older-format file could also carry non-str values, or
        # str values with embedded CR/LF that would crash urllib's HTTP-header
        # writer (RFC 7230 forbids CR/LF in field-values). Validate the
        # container, the value type, and the absence of CR/LF; silently drop
        # anything off-shape — the next 200 OK response will rewrite the meta.
        if not isinstance(meta, dict):
            meta = {}
        et = meta.get("etag")
        lm = meta.get("last_modified")
        if isinstance(et, str) and "\r" not in et and "\n" not in et:
            headers["If-None-Match"] = et
        if isinstance(lm, str) and "\r" not in lm and "\n" not in lm:
            headers["If-Modified-Since"] = lm

    req = urllib.request.Request(url, headers=headers)
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            data = resp.read()
            meta = {}
            etag = resp.headers.get("ETag")
            last_mod = resp.headers.get("Last-Modified")
            if etag:
                meta["etag"] = etag
            if last_mod:
                meta["last_modified"] = last_mod
            # Cache writes are best-effort: if the disk fills mid-fetch or
            # the partition turns read-only, still return the freshly
            # downloaded bytes to the caller.
            try:
                body_path.write_bytes(data)
                meta_path.write_text(json.dumps(meta), encoding="utf-8")
            except OSError:
                pass
            return data
    except urllib.error.HTTPError as e:
        if e.code == 304:
            if body_path.exists():
                try:
                    return body_path.read_bytes()
                except OSError:
                    pass  # fall through to unconditional refetch
            # Cache eviction race: meta survived but body is gone (manual rm,
            # cleanup tool between runs, etc.). Drop the stale meta and recurse
            # — the next call has no conditional headers, so it does a normal
            # 200 fetch and rewrites both body and meta. The fallback to
            # `_raw_fetch` (without cache write) covers the read-only-fs case
            # where unlink fails; without that guard, the recursion would
            # loop on the same conditional GET.
            try:
                meta_path.unlink(missing_ok=True)
            except OSError:
                pass
            if not meta_path.exists():
                return cached_fetch_url(url)
            return _raw_fetch(url, headers={})
        if e.code == 404:
            raise NotFound(url) from e
        sys.exit(f"webref: HTTP {e.code} fetching {url}")
    except urllib.error.URLError as e:
        sys.exit(f"webref: network error fetching {url}: {e.reason}")


def _raw_fetch(url: str, headers: dict[str, str]) -> bytes:
    req = urllib.request.Request(url, headers=headers)
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            return resp.read()
    except urllib.error.HTTPError as e:
        if e.code == 404:
            raise NotFound(url) from e
        sys.exit(f"webref: HTTP {e.code} fetching {url}")
    except urllib.error.URLError as e:
        sys.exit(f"webref: network error fetching {url}: {e.reason}")
