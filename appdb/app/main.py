"""FastAPI server for the touchHLE app compatibility database."""
from __future__ import annotations

import re
from collections import defaultdict
from datetime import datetime
from pathlib import Path
from typing import Annotated

from fastapi import Depends, FastAPI, Form, HTTPException, Request
from fastapi.responses import HTMLResponse, RedirectResponse
from fastapi.staticfiles import StaticFiles
from fastapi.templating import Jinja2Templates
from sqlalchemy import func
from sqlalchemy.orm import Session
from starlette.middleware.trustedhost import TrustedHostMiddleware  # noqa: F401  (kept for future use)
from uvicorn.middleware.proxy_headers import ProxyHeadersMiddleware

from .db import App, Report, get_db, init_db
from .seed import seed

BASE_DIR = Path(__file__).resolve().parent
templates = Jinja2Templates(directory=str(BASE_DIR / "templates"))

RATING_LEGEND = [
    (1, "Completely broken: app crashes immediately without any user interaction."),
    (2, "Only (part of) the main menu, intro or similar is working."),
    (3, "Some of the main content of the app works, but with major issues."),
    (4, "The main content of the app works (e.g. entire game is playable) with only small issues."),
    (5, "Everything works. The app is fully usable."),
]

SCALE_HACK_CHOICES = ["Yes", "No", "Partially", "Couldn't test", "Didn't test"]


def stars(rating: int | None) -> str:
    if rating is None:
        return "—"
    rating = max(1, min(5, int(rating)))
    return "⭐" * rating


def _format_dt(dt: datetime | None) -> str:
    if dt is None:
        return ""
    return dt.strftime("%Y-%m-%d %H:%M:%S")


templates.env.filters["stars"] = stars
templates.env.filters["fmt_dt"] = _format_dt


app = FastAPI(title="touchHLE app compatibility database")
# Trust X-Forwarded-* headers when running behind a TLS-terminating proxy (e.g. Fly).
# Without this, ``request.url`` reports ``http://`` and ``url_for`` emits broken
# URLs that browsers block as mixed content.
app.add_middleware(ProxyHeadersMiddleware, trusted_hosts="*")
app.mount("/static", StaticFiles(directory=str(BASE_DIR / "static")), name="static")


@app.on_event("startup")
def _startup() -> None:
    init_db()
    seed()


@app.get("/healthz")
def healthz() -> dict[str, str]:
    return {"status": "ok"}


@app.get("/", response_class=HTMLResponse)
def home(
    request: Request,
    q: str | None = None,
    db: Annotated[Session, Depends(get_db)] = None,
):
    apps = db.query(App).order_by(App.name).all()

    # Compute best rating + last_updated per app.
    best_rating: dict[int, int | None] = {}
    last_updated: dict[int, datetime | None] = {}
    rows = (
        db.query(
            Report.app_id,
            func.max(Report.rating).label("best"),
            func.max(Report.reported_at).label("last_at"),
        )
        .group_by(Report.app_id)
        .all()
    )
    for app_id, best, last_at in rows:
        best_rating[app_id] = best
        last_updated[app_id] = last_at

    # Stats per rating (count apps that have at least one report at that rating
    # as their best rating).
    rating_counts = defaultdict(int)
    for app_id, best in best_rating.items():
        if best is not None:
            rating_counts[best] += 1

    # Filter
    if q:
        ql = q.lower().strip()
        apps = [
            a
            for a in apps
            if ql in a.name.lower()
            or (a.developer_publisher and ql in a.developer_publisher.lower())
        ]

    return templates.TemplateResponse(
        request,
        "index.html",
        {
            "apps": apps,
            "best_rating": best_rating,
            "last_updated": last_updated,
            "rating_counts": rating_counts,
            "rating_legend": RATING_LEGEND,
            "q": q or "",
            "total_apps": len(apps),
        },
    )


@app.get("/apps/{app_id}", response_class=HTMLResponse)
def app_detail(
    app_id: int,
    request: Request,
    db: Annotated[Session, Depends(get_db)] = None,
):
    a = db.query(App).filter(App.id == app_id).first()
    if a is None:
        raise HTTPException(404, "App not found")

    reports = (
        db.query(Report)
        .filter(Report.app_id == app_id)
        .order_by(Report.reported_at.desc())
        .all()
    )

    # Group reports by version_number to build the "Versions" summary table.
    versions: dict[str, dict] = {}
    for r in reports:
        v = versions.setdefault(
            r.version_number,
            {
                "version_number": r.version_number,
                "display_name": r.display_name,
                "bundle_identifier": r.bundle_identifier,
                "minimum_ios_version": r.minimum_ios_version,
                "best_rating": r.rating,
                "last_updated": r.reported_at,
                "first_reported_by": r.reported_by,
                "first_reported_at": r.reported_at,
            },
        )
        if r.rating > v["best_rating"]:
            v["best_rating"] = r.rating
        if r.reported_at > v["last_updated"]:
            v["last_updated"] = r.reported_at
        if r.reported_at < v["first_reported_at"]:
            v["first_reported_at"] = r.reported_at
            v["first_reported_by"] = r.reported_by
        # Prefer non-empty bundle / display info if present.
        if not v["display_name"] and r.display_name:
            v["display_name"] = r.display_name
        if not v["bundle_identifier"] and r.bundle_identifier:
            v["bundle_identifier"] = r.bundle_identifier
        if not v["minimum_ios_version"] and r.minimum_ios_version:
            v["minimum_ios_version"] = r.minimum_ios_version

    versions_sorted = sorted(versions.values(), key=lambda v: v["version_number"])

    return templates.TemplateResponse(
        request,
        "app_detail.html",
        {
            "app": a,
            "reports": reports,
            "versions": versions_sorted,
            "rating_legend": RATING_LEGEND,
        },
    )


@app.get("/submit", response_class=HTMLResponse)
def submit_form(
    request: Request,
    app_id: int | None = None,
    db: Annotated[Session, Depends(get_db)] = None,
):
    apps = db.query(App).order_by(App.name).all()
    selected = None
    if app_id is not None:
        selected = db.query(App).filter(App.id == app_id).first()
    return templates.TemplateResponse(
        request,
        "submit_report.html",
        {
            "apps": apps,
            "selected_app": selected,
            "rating_legend": RATING_LEGEND,
            "scale_hack_choices": SCALE_HACK_CHOICES,
            "error": None,
            "form": {},
        },
    )


def _clean(s: str | None) -> str | None:
    if s is None:
        return None
    s = s.strip()
    return s or None


_USERNAME_RE = re.compile(r"^@?[A-Za-z0-9_\-]{1,40}$")


@app.post("/submit", response_class=HTMLResponse)
def submit_post(
    request: Request,
    app_id: Annotated[str, Form()],
    new_app_name: Annotated[str, Form()] = "",
    new_app_year: Annotated[str, Form()] = "",
    new_app_publisher: Annotated[str, Form()] = "",
    version_number: Annotated[str, Form()] = "",
    display_name: Annotated[str, Form()] = "",
    bundle_identifier: Annotated[str, Form()] = "",
    minimum_ios_version: Annotated[str, Form()] = "",
    touchhle_version: Annotated[str, Form()] = "",
    operating_system: Annotated[str, Form()] = "",
    gpu: Annotated[str, Form()] = "",
    scale_hack: Annotated[str, Form()] = "",
    rating: Annotated[str, Form()] = "",
    remarks: Annotated[str, Form()] = "",
    screenshot_url: Annotated[str, Form()] = "",
    reported_by: Annotated[str, Form()] = "",
    db: Annotated[Session, Depends(get_db)] = None,
):
    form_data = {
        "app_id": app_id,
        "new_app_name": new_app_name,
        "new_app_year": new_app_year,
        "new_app_publisher": new_app_publisher,
        "version_number": version_number,
        "display_name": display_name,
        "bundle_identifier": bundle_identifier,
        "minimum_ios_version": minimum_ios_version,
        "touchhle_version": touchhle_version,
        "operating_system": operating_system,
        "gpu": gpu,
        "scale_hack": scale_hack,
        "rating": rating,
        "remarks": remarks,
        "screenshot_url": screenshot_url,
        "reported_by": reported_by,
    }

    def _err(msg: str) -> HTMLResponse:
        apps = db.query(App).order_by(App.name).all()
        selected = None
        if app_id and app_id != "__new__":
            try:
                selected = db.query(App).filter(App.id == int(app_id)).first()
            except ValueError:
                selected = None
        return templates.TemplateResponse(
            request,
            "submit_report.html",
            {
                "apps": apps,
                "selected_app": selected,
                "rating_legend": RATING_LEGEND,
                "scale_hack_choices": SCALE_HACK_CHOICES,
                "error": msg,
                "form": form_data,
            },
            status_code=400,
        )

    # Resolve / create app.
    if app_id == "__new__":
        name = _clean(new_app_name)
        if not name:
            return _err("Please enter a name for the new app.")
        existing = db.query(App).filter(App.name == name).first()
        if existing:
            target_app = existing
        else:
            year_val: int | None = None
            if _clean(new_app_year):
                try:
                    year_val = int(new_app_year)
                except ValueError:
                    return _err("Release year must be a number.")
            target_app = App(
                name=name,
                release_year=year_val,
                developer_publisher=_clean(new_app_publisher),
                first_reported_by=_clean(reported_by),
            )
            db.add(target_app)
            db.flush()
    else:
        try:
            target_app = db.query(App).filter(App.id == int(app_id)).first()
        except ValueError:
            target_app = None
        if target_app is None:
            return _err("Please choose an existing app or pick “Add a new app”.")

    # Validate report fields.
    if not _clean(version_number):
        return _err("Version number is required.")
    if not _clean(touchhle_version):
        return _err("touchHLE version is required.")
    if not _clean(operating_system):
        return _err("Operating system is required.")
    try:
        rating_val = int(rating)
    except (TypeError, ValueError):
        return _err("Rating is required.")
    if rating_val < 1 or rating_val > 5:
        return _err("Rating must be 1–5.")

    reporter = _clean(reported_by)
    if reporter and not _USERNAME_RE.match(reporter):
        return _err("“Reported by” should be a username (letters, digits, _ or -, optionally prefixed with @).")
    if reporter and not reporter.startswith("@"):
        reporter = "@" + reporter

    sh = _clean(scale_hack)
    if sh and sh not in SCALE_HACK_CHOICES:
        return _err("Invalid scale-hack value.")

    report = Report(
        app_id=target_app.id,
        version_number=_clean(version_number) or "",
        display_name=_clean(display_name),
        bundle_identifier=_clean(bundle_identifier),
        minimum_ios_version=_clean(minimum_ios_version),
        touchhle_version=_clean(touchhle_version) or "",
        operating_system=_clean(operating_system) or "",
        gpu=_clean(gpu),
        scale_hack=sh,
        rating=rating_val,
        remarks=_clean(remarks),
        screenshot_url=_clean(screenshot_url),
        reported_by=reporter,
    )
    db.add(report)
    db.commit()

    return RedirectResponse(url=f"/apps/{target_app.id}", status_code=303)


@app.get("/about", response_class=HTMLResponse)
def about(request: Request):
    return templates.TemplateResponse(
        request,
        "about.html",
        {"rating_legend": RATING_LEGEND},
    )
