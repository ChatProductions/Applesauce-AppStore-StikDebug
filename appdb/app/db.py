"""SQLAlchemy models and DB init for the touchHLE app compatibility database."""
from __future__ import annotations

import os
from datetime import datetime
from pathlib import Path

from sqlalchemy import (
    Column,
    DateTime,
    ForeignKey,
    Integer,
    String,
    Text,
    create_engine,
    func,
)
from sqlalchemy.orm import DeclarativeBase, Mapped, mapped_column, relationship, sessionmaker


def _database_url() -> str:
    """Return the database URL.

    Uses ``DATABASE_URL`` if set (e.g. for Postgres), otherwise a SQLite file
    on a persistent volume in production (``/data/appdb.sqlite3``) or in the
    project root in development.
    """
    url = os.environ.get("DATABASE_URL")
    if url:
        return url
    data_dir = Path("/data") if Path("/data").is_dir() else Path(__file__).resolve().parent.parent
    data_dir.mkdir(parents=True, exist_ok=True)
    return f"sqlite:///{data_dir / 'appdb.sqlite3'}"


class Base(DeclarativeBase):
    pass


class App(Base):
    __tablename__ = "apps"

    id: Mapped[int] = mapped_column(Integer, primary_key=True)
    name: Mapped[str] = mapped_column(String(200), nullable=False, unique=True, index=True)
    release_year: Mapped[int | None] = mapped_column(Integer, nullable=True)
    developer_publisher: Mapped[str | None] = mapped_column(String(200), nullable=True)
    first_reported_at: Mapped[datetime] = mapped_column(
        DateTime, default=datetime.utcnow, nullable=False
    )
    first_reported_by: Mapped[str | None] = mapped_column(String(80), nullable=True)

    reports: Mapped[list[Report]] = relationship(
        "Report", back_populates="app", cascade="all, delete-orphan"
    )


class Report(Base):
    __tablename__ = "reports"

    id: Mapped[int] = mapped_column(Integer, primary_key=True)
    app_id: Mapped[int] = mapped_column(ForeignKey("apps.id"), nullable=False, index=True)

    # Version-level info (denormalised for simplicity — we group by version_number on display).
    version_number: Mapped[str] = mapped_column(String(40), nullable=False, index=True)
    display_name: Mapped[str | None] = mapped_column(String(120), nullable=True)
    bundle_identifier: Mapped[str | None] = mapped_column(String(200), nullable=True)
    minimum_ios_version: Mapped[str | None] = mapped_column(String(20), nullable=True)

    # Report-level info.
    touchhle_version: Mapped[str] = mapped_column(String(80), nullable=False)
    operating_system: Mapped[str] = mapped_column(String(120), nullable=False)
    gpu: Mapped[str | None] = mapped_column(String(120), nullable=True)
    scale_hack: Mapped[str | None] = mapped_column(String(40), nullable=True)
    # Rating 1..5, where 1=completely broken and 5=fully working.
    rating: Mapped[int] = mapped_column(Integer, nullable=False)
    remarks: Mapped[str | None] = mapped_column(Text, nullable=True)
    screenshot_url: Mapped[str | None] = mapped_column(String(500), nullable=True)

    reported_at: Mapped[datetime] = mapped_column(
        DateTime, default=datetime.utcnow, nullable=False, index=True
    )
    reported_by: Mapped[str | None] = mapped_column(String(80), nullable=True)

    app: Mapped[App] = relationship("App", back_populates="reports")


engine = create_engine(
    _database_url(),
    connect_args={"check_same_thread": False} if _database_url().startswith("sqlite") else {},
    pool_pre_ping=True,
)
SessionLocal = sessionmaker(autocommit=False, autoflush=False, bind=engine)


def init_db() -> None:
    Base.metadata.create_all(bind=engine)


def get_db():
    db = SessionLocal()
    try:
        yield db
    finally:
        db.close()


__all__ = ["App", "Report", "Base", "engine", "SessionLocal", "init_db", "get_db", "func"]
