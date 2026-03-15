"""A Python module for querying JSON data using JSONPath expressions."""

from typing import Optional

def find(path: str, data: object) -> Optional[str]:
    """Find the value(s) in the JSON data that match the given JSONPath expression."""