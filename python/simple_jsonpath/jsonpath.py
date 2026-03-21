from ._simple_jsonpath import SimpleJsonPath as RustSimpleJsonPath
import sys
if sys.version_info >= (3, 11):
    from typing import Self, Union, Any
else:
    from typing_extensions import Self, Union, Any
import orjson
from dataclasses import dataclass
import builtins
import json

class PathComponentsIter:
    def __init__(self, path: str, nodes: list[tuple[int, int]]):
        self._current: int = 0
        self._path: str = path    
        self._end: int = len(nodes)
        self._items: list[tuple[int,int]] = nodes

    def __next__(self) -> Union[str, int]:
        if self._current >= self._end:
            raise StopIteration
        if self._current == 0:
            self._current +=1
            return "$"
        else:
            item = self._items[self._current]
            if item[0] == 0:
                self._current += 1
                return item[1]
            else:
                self._current += 1
                return self._path[item[0]:item[1]]

@dataclass(frozen=True)
class PathComponents:
    _path: str
    _items: list[tuple[int, int]]

    def __len__(self) -> int:
        return len(self._items)

    def __iter__(self) -> PathComponentsIter:
        return PathComponentsIter(self._path, self._items)
    
    def __getitem__(self, index: int) -> Union[str, int]:
        if index >= len(self._items):
            raise IndexError("Index out of range")
        elif index == 0:
            return "$"
        elif index < 0:
            raise IndexError("Negative indexing is not supported")  
        else:
            item = self._items[index]
            if item[0] == 0:
                return item[1]
            else:
                return self._path[item[0]:item[1]]
    def __contains__(self, item: Union[int, str]) -> bool:
        if isinstance(item, int):
            for i in range(len(self._items)):
                if i == 0:
                    continue
                else:
                    if self._items[i][0] == 0 and self._items[i][1] == item:
                        return True
            return False
        else:
            for i in range(len(self._items[1:])):
                if i == 0:
                    if item == "$":
                        return True
                else:   
                    if self._path[self._items[i][0]:self._items[i][1]] == item:
                        return True
            return False
          
class LocatedNode:
    """A struct to hold the located nodes found from a located JSONPath query."""
    def __init__(self, full_path: str, path_components: list[tuple[int, int]], node: Union[str,int,float,bool,None,dict[str, Any], list[Any]]) -> None:
        self._full_path: str = full_path
        self._path_components: PathComponents = PathComponents(full_path, path_components)
        self._node: Union[str,int,float,bool,None,dict[str, Any], list[Any]] = node

    @builtins.property
    def path_components(self) -> PathComponents:
        """An iterator that yields the path components of the last query result."""
        return self._path_components
    @builtins.property
    def full_path(self) -> str:
        """The full path of the last query result."""
        return self._full_path
    @builtins.property   
    def node(self) -> Union[str,int,float,bool,None,dict[str, Any], list[Any]]:
        """The node value of the last query result."""
        return self._node


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
            path: The JSONPath expression to evaluate.

        Returns:
            A list of values that match the JSONPath expression.

        Raises:
            ValueError: If the JSONPath expression is invalid.
            LookupError: If this is called before data has not been set to this object through 'set_data()'.
        """
        if not self._parser.has_data():
            raise LookupError("Data must be set through calling 'set_data()' before attempting a query")
        return self._parser.find(path)

    def find_located(self, path: str) -> list[LocatedNode]:
        """Find the value(s) in the JSON data that match the given JSONPath expression, along with their locations.

        The path expression is first parsed, then executed against the data previously
        'set'. Parsed path expressions are cached for efficient future use.
        
        Args:
            path: The JSONPath expression to evaluate.

        Returns:
            A list of LocatedNode objects that match the JSONPath expression.

        Raises:
            ValueError: If the JSONPath expression is invalid.
            LookupError: If this is called before data has not been set to this object through 'set_data()'.
        """
        if not self._parser.has_data():
            raise LookupError("Data must be set through calling 'set_data()' before attempting a query")
        result = self._parser.find_located(path)
        return [LocatedNode(item['full_path'], item['path_components'], item['node']) for item in result]

