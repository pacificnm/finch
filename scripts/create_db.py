#!/usr/bin/env python3
"""Create the Finch PostgreSQL database if it does not already exist.

Expects DATABASE_URL in the environment (sourced from .env by start-dev.sh).
"""

import os
import re
import sys
import urllib.parse


def parse_url(url: str) -> dict:
    parsed = urllib.parse.urlparse(url)
    if parsed.scheme not in ("postgresql", "postgres"):
        raise ValueError(f"unsupported scheme: {parsed.scheme}")
    return {
        "host": parsed.hostname or "localhost",
        "port": parsed.port or 5432,
        "user": parsed.username or "postgres",
        "password": parsed.password or "",
        "dbname": parsed.path.lstrip("/") or "finch",
    }


def main() -> int:
    url = os.environ.get("DATABASE_URL")
    if not url:
        print("DATABASE_URL is not set", file=sys.stderr)
        return 1

    cfg = parse_url(url)
    target_db = cfg["dbname"]
    admin_db = "postgres"

    try:
        import psycopg2
    except ImportError:
        print(
            "psycopg2 is required. Install it in a venv: pip install psycopg2-binary",
            file=sys.stderr,
        )
        return 1

    conn_str = (
        f"host={cfg['host']} port={cfg['port']} dbname={admin_db} "
        f"user={cfg['user']} password={cfg['password']}"
    )

    conn = psycopg2.connect(conn_str)
    conn.autocommit = True
    cur = conn.cursor()
    cur.execute("SELECT 1 FROM pg_database WHERE datname = %s", (target_db,))
    if cur.fetchone():
        print(f"Database '{target_db}' already exists.")
    else:
        # Identifier is user-controlled from config; quote it safely.
        cur.execute(f'CREATE DATABASE "{target_db}"')
        print(f"Created database '{target_db}'.")

    cur.close()
    conn.close()
    return 0


if __name__ == "__main__":
    sys.exit(main())
