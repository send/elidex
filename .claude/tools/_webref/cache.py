"""HTTP cache shared by webref + tc39 biblio paths.

Conditional GET (ETag / Last-Modified) against ~/.cache/elidex-webref/ (or
$XDG_CACHE_HOME/elidex-webref/). Cache is an optimization, not a correctness
layer — any I/O failure falls back to uncached fetch.
"""
from __future__ import annotations

import hashlib
import http.client
import json
import os
import urllib.error
import datetime
import email.utils
import urllib.request
from pathlib import Path
from typing import NamedTuple


class FetchError(Exception):
    """An HTTP / network failure fetching a URL. `str(e)` is the user-facing
    `webref: ...` message; the CLI edge (`cli.main`) turns it into the exit
    status, so library code can catch it and try another source."""


class NotFound(FetchError):
    """HTTP 404. `str(e)` is the URL (the CLI edge words it `fetch failed:`)."""


class Fetched(NamedTuple):
    """A fetched copy of a URL with its provenance. `date` belongs to this
    copy, never to cache state read afterwards: the response's Last-Modified
    when it sent one, else its Date header, else the fetch time (today); after
    a 304 the stored copy's Last-Modified if it had one, else today (the day
    the copy was revalidated)."""
    data: bytes
    date: datetime.date


def _http_date(value: object) -> datetime.date | None:
    """The date of an HTTP-date header value; None when absent or unparsable."""
    if not isinstance(value, str):
        return None
    try:
        dt = email.utils.parsedate_to_datetime(value)
        if dt.tzinfo is not None:
            dt = dt.astimezone(datetime.timezone.utc)
        return dt.date()
    except (ValueError, TypeError, OverflowError):
        return None


def _response_date(headers: object) -> datetime.date:
    """Date of a 200 response: Last-Modified, else Date, else today."""
    for name in ("Last-Modified", "Date"):
        d = _http_date(headers.get(name)) if hasattr(headers, "get") else None
        if d is not None:
            return d
    return datetime.date.today()


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
    """`cached_fetch(url).data`: for callers that do not need the provenance."""
    return cached_fetch(url).data


def cached_fetch(url: str) -> Fetched:
    """GET `url` with conditional-GET caching, returning the bytes together
    with the date of that copy (see `Fetched`).

    Stores the response body alongside ETag / Last-Modified in
    ~/.cache/elidex-webref/ (or $XDG_CACHE_HOME/elidex-webref/ when set).
    On subsequent calls sends If-None-Match / If-Modified-Since; on 304
    returns the cached body. Set ELIDEX_WEBREF_NO_CACHE=1 to bypass
    entirely (uncached fetch). If the cache directory or any cache file
    is unwritable, transparently falls back to uncached fetch — cache is
    an optimization, not a correctness layer.

    Raises NotFound on 404, FetchError on other network / HTTP errors.
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
    stored_lm: datetime.date | None = None
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
            stored_lm = _http_date(lm)

    req = urllib.request.Request(url, headers=headers)
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            data = resp.read()
            date = _response_date(resp.headers)
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
            return Fetched(data, date)
    except urllib.error.HTTPError as e:
        if e.code == 304:
            if body_path.exists():
                try:
                    return Fetched(body_path.read_bytes(),
                                   stored_lm or datetime.date.today())
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
                return cached_fetch(url)
            return _raw_fetch(url, headers={})
        if e.code == 404:
            raise NotFound(url) from e
        raise FetchError(f"webref: HTTP {e.code} fetching {url}") from e
    except urllib.error.URLError as e:
        raise FetchError(f"webref: network error fetching {url}: {e.reason}") from e
    except (TimeoutError, OSError, http.client.HTTPException) as e:
        # Socket timeout / reset / RemoteDisconnected raised from
        # getresponse() or read(): urllib wraps only connect-phase errors.
        raise FetchError(f"webref: network error fetching {url}: {e}") from e


def _raw_fetch(url: str, headers: dict[str, str]) -> Fetched:
    req = urllib.request.Request(url, headers=headers)
    try:
        with urllib.request.urlopen(req, timeout=30) as resp:
            return Fetched(resp.read(), _response_date(resp.headers))
    except urllib.error.HTTPError as e:
        if e.code == 404:
            raise NotFound(url) from e
        raise FetchError(f"webref: HTTP {e.code} fetching {url}") from e
    except urllib.error.URLError as e:
        raise FetchError(f"webref: network error fetching {url}: {e.reason}") from e
    except (TimeoutError, OSError, http.client.HTTPException) as e:
        # Socket timeout / reset / RemoteDisconnected raised from
        # getresponse() or read(): urllib wraps only connect-phase errors.
        raise FetchError(f"webref: network error fetching {url}: {e}") from e
