from ._simple_jsonpath import SimpleJsonPath as RustSimpleJsonPath
from typing import Any, Union
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

    def set_data(self, input_data: Union[dict[str, Any], list[Any]]) -> None:
        """Set the JSON data for the parser from a Python dictionary or list.
        
        Args:
            input_data: The JSON data to set, as a Python dictionary or list.

        Returns:
            None

        Raises:
            ValueError: If the input data is not a valid JSON object or array.
        """
        self._parser.set_data_from_json_str(json.dumps(input_data))

    def find(self, path: str) -> list[Any]:
        """Find the value(s) in the JSON data that match the given JSONPath expression.
        
        Args:
            path: The JSONPath expression to evaluate.

        Returns:
            A list of values that match the JSONPath expression.

        Raises:
            ValueError: If the JSONPath expression is invalid.
        """
        return json.loads(self._parser.find_from_set_data(path))

    def find_located(self, path: str) -> list[LocatedNode]:
        """Find the value(s) in the JSON data that match the given JSONPath expression, along with their locations.
        
        Args:
            path: The JSONPath expression to evaluate.

        Returns:
            A list of LocatedNode objects that match the JSONPath expression.

        Raises:
            ValueError: If the JSONPath expression is invalid.
        """
        result = json.loads(self._parser.find_location_from_set_data(path))
        return [LocatedNode(item['full_path'], item['path_components'], item['node']) for item in result]
