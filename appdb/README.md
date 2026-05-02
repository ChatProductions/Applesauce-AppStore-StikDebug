# touchHLE app compatibility database

A community-run compatibility database for [this fork of touchHLE](https://github.com/j92580498-max/touchHLE),
modelled on the original [appdb.touchhle.org](https://appdb.touchhle.org/).

Anyone can submit a compatibility report for an app they've tested in
touchHLE: app name, version, OS, GPU, rating (1–5⭐), remarks, screenshot.
The site aggregates them per app and per app version.

## Stack

- **FastAPI** + **Jinja2** (server-rendered HTML, no SPA)
- **SQLite** via SQLAlchemy 2.x
- Plain CSS, no build step

## Run locally

```bash
cd appdb
python -m venv .venv
source .venv/bin/activate
pip install -e .
uvicorn app.main:app --host 0.0.0.0 --port 8000 --reload
```

Then open <http://localhost:8000/>. The database is auto-created and seeded
with a handful of example apps the first time the server starts.

The SQLite file lives at `appdb/appdb.sqlite3` in development. In
production (Fly.io with a volume) it lives at `/data/appdb.sqlite3`.

## Layout

```
appdb/
├── pyproject.toml         # dependencies + package metadata
├── README.md
└── app/
    ├── main.py            # FastAPI routes
    ├── db.py              # SQLAlchemy models, engine, init_db
    ├── seed.py            # demo data
    ├── templates/         # Jinja2 templates
    │   ├── base.html
    │   ├── index.html         # Apps list + per-rating stats
    │   ├── app_detail.html    # Per-app: versions + reports table
    │   ├── submit_report.html # Form to add a compatibility report
    │   └── about.html
    └── static/
        └── style.css
```

## Deploying

The app is deployable as a FastAPI backend (e.g. to Fly.io with a 1 GB
volume mounted at `/data` for the SQLite file).

## Routes

| Method | Path | Description |
| --- | --- | --- |
| GET | `/` | Apps list + stats, optional `?q=` search |
| GET | `/apps/{id}` | App detail page (versions + reports) |
| GET | `/submit` | Compatibility report form |
| POST | `/submit` | Create a new report (and optionally a new app) |
| GET | `/about` | Rating scale and house rules |
| GET | `/healthz` | Liveness check |

## Notes

- This is **not** affiliated with upstream touchHLE; it is a community
  database for this specific fork.
- Reports are anonymous — there is no login. To prevent spam in
  production, you'll want to add a CAPTCHA or rate limiting before
  exposing it widely.
