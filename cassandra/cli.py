"""Backwards compatibility - CLI moved to cassandra.cli package."""

from cassandra.cli import main

if __name__ == "__main__":
    main()
