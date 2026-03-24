from ._simple_jsonpath import SimpleJsonPath as RustSimpleJsonPath, Path
import sys

if sys.version_info >= (3, 11):
    from typing import Self, Union, Any, Optional
else:
    from typing_extensions import Self, Union, Any, Optional
import orjson
import builtins


class LocatedNode:
    """A class that represents a node found in the JSON data along with its location path."""

    def __init__(
        self,
        path: Path,
        node: Union[str, int, float, bool, None, dict[str, Any], list[Any]],
    ) -> None:
        self._path: Path = path
        self._node: Union[str, int, float, bool, None, dict[str, Any], list[Any]] = node

    @builtins.property
    def path(self) -> Path:
        """An iterator that yields the path components of the last query result.

        The full path can be converted to a str through the str() method against this object.
        """
        return self._path

    @builtins.property
    def node(self) -> Union[str, int, float, bool, None, dict[str, Any], list[Any]]:
        """The node value of the last query result."""
        return self._node
    
    @builtins.property
    def parent_path(self) -> Optional[Path]:
        """The parent path of the last query result."""
        return self._path.parent_path()


class JsonPath:
    """A simple JSONPath implementation for querying JSON data.

    It uses a Rust backend for performance and supports caching of parsed
    JSONPath expressions for repeated queries against the same JSON data.
    """

    def __init__(self) -> None:
        self._parser = RustSimpleJsonPath()

    def child(self) -> Self:
        """Spawns a child instance of the class.

        The child will not inherit the data of the parent, so a call
        to set_data() need to be called on it for it to function.

        It does however retain shared mutable access to the parent's collection
        of pre-parsed path objects across all spawned children.

        This is useful for the pattern of:
            1. Searching a document for a path query.
            2. Then using those results returned as the basis of a new 'root element'
            for 'deeper' searches into a document.

        Instead of assigning the original query results to current instance, it can
        be beneficial to spawn a child for each result, and assign the result
        data to the child or multiple children if more than one result was returned.

        With this pattern the 'base' parent object will automatically contain
        all parsed paths for the document that were searched by
        any descendant instance spawned from it.

        Then the 'base' parent object can be efficiently used on the next similarly
        structured document as all previously complied queries are retained.
        """
        # Jesus this took too long to figure out...
        child_cls = self.__class__
        child = child_cls()
        return child

    def has_data(self) -> bool:
        """Returns True if this instance has data set to it, False otherwise."""
        return self._parser.has_data()

    def set_data(self, input_data: Union[dict[str, Any], list[Any]]) -> None:
        """Set the JSON data for the query engine from a Python object.

        Once set, this data will be used for any find() or find_located()
        operations performed against this instance.

        Calling the function consecutively will replace any existing 'set' data.

        Args:
            input_data: The JSON data to set, as a Python dictionary or list.

        Returns:
            None

        Raises:
            ValueError: If the input data is not a valid JSON object or array.
        """
        self._parser.set_data(orjson.dumps(input_data))

    def find(self, path: str) -> list[Any]:
        """Find the value(s) in the JSON data that match the given JSONPath expression.

        The path expression is first parsed, then executed against the data previously
        'set'. Parsed path expressions are cached for efficient future use.

        Args:
            path(str): The JSONPath expression to evaluate.

        Returns:
            list[Any]: A list of values that match the JSONPath expression.

        Raises:
            ValueError: If the JSONPath expression is invalid.
            LookupError: If this is called before data has not been set to this object through 'set_data()'.
        """
        if not self._parser.has_data():
            raise LookupError(
                "Data must be set through calling 'set_data()' before attempting a query"
            )
        return self._parser.find(path)

    def find_located(self, path: str) -> list[LocatedNode]:
        """Find the value(s) in the JSON data that match the given JSONPath expression, along with their locations.

        The path expression is first parsed, then executed against the data previously
        'set'. Parsed path expressions are cached for efficient future use.

        Args:
            path(str): The JSONPath expression to evaluate.

        Returns:
            list[LocatedNode]: A list of 'LocatedNode' objects that match the JSONPath expression.

        Raises:
            ValueError: If the JSONPath expression is invalid.
            LookupError: If this is called before data has not been set to this object through 'set_data()'.
        """
        if not self._parser.has_data():
            raise LookupError(
                "Data must be set through calling 'set_data()' before attempting a query"
            )
        result = self._parser.find_located(path)
        return [LocatedNode(path, node) for (path, node) in result]
