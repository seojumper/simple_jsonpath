"""A Python module for querying JSON data using JSONPath expressions."""


class SimpleJsonPath:
    """A parser object that can be reused for multiple queries on the same JSON data."""
    def __init__(self) -> None: ...
    def set_data_from_json_str(self, input_data: str) -> None:
        """Set the JSON data for the parser from a JSON string."""
        ...
    def find_from_set_data(self, path: str) -> str: 
        """Find the value(s) in the JSON data that match the given JSONPath expression, using a cache for parsed paths."""
        ...

    def find_location_from_set_data(self, path: str) -> str:
        """Find the value(s) in the JSON data that match the given JSONPath expression, along with their locations, using a cache for parsed paths."""
        ...