# simple_jsonpath

## Installation

```bash
pip install simple_jsonpath
```

## About

This module is a JSONPath [RFC9535 - JSONPath: Query Expressions for JSON](https://datatracker.ietf.org/doc/html/rfc9535) utility library.

## Use

This module exposes a single simple type - a ***JsonPath*** which has two methods after instantiation.

- ***set_data()***: sets the data that will be queried against. If multiple queries will be performed against a single piece of JSON data, this helps with the type conversion cost involved. This function can be called with providing new data whenever the inner data held is wished to be changed while retaining already complied paths (useful for querying multiple similarly structured documents).
- ***find()***: given a path that is wished to be found in the previously set data, this function will perform the query logic. Mulitple calls to **find()** will query against the previously 'set' data.
- ***find_located()***: given a path that is wished to be found in the previously set data, this function can return a list of ***LocatedNode*** objects.  Each ***LocatedNode*** object will have attributes related to the path where the node was located as well as their corresponding data. This method is slower than ***find()***, so should ideally only be used when path information for the found nodes is needed.

## Examples

### 'Find' Example

```python
from simple_jsonpath import JsonPath


json_data = {
    "address": {
        "prefix-list": [
            {
                "prefix": "2001:db8::1/64",
                "eui-64": [
                    None
                ]
            }
        ],
        "link-local-address": [
            {
                "address": "fe80::1",
                "link-local": [
                    None
                ]
            }
        ]
    }
}

# Instantiates the primary class
finder = JsonPath()

# Sets the data that is desired to be queried against
finder.set_data(json_data)

# A path is provided to query against the 'set' data.  The path is internally parsed > used to qeury against the 'set' dataset.
# Notice that this implementaion allows for escaping of specials characters shorthand path syntax with single or double quotes
results = finder.find("$.address.'prefix-list'[*].prefix")

for data in results:
    # Access the found node. 
    print(f"{data}")
    # 2001:db8::1/64

```

The inner implementation stores previously parsed 'paths'. This allows repeatedly used paths to bypass the parsing step invovled.

This is ideal for situations where multiple similar JSON documents will be searched in succession.

The same **JsonPath** object can then be reused with new data sets by calling **set_data()** on it again, and any previously parsed paths by the object will be retained.

Only when moving onto data of differing structure would it be potentially advisable to instantiate a new **JsonPath** object.

### "Find Located' Example

```python
from simple_jsonpath import JsonPath, LocatedNode


json_data = {
    "items": [
        {
            "address": {
                "prefix-list": [
                    {
                        "prefix": "2001:db8::1/64",
                        "eui-64": [
                            None
                        ]
                    }
                ],
                "link-local-address": [
                    {
                        "address": "fe80::1",
                        "link-local": [
                            None
                        ]
                    }
                ]
            }
        },
        {
            "address": {
                "prefix-list": [
                    {
                        "prefix": "2001:db8::1/64",
                        "eui-64": [
                            None
                        ]
                    }
                ],
                "link-local-address": [
                    {
                        "address": "fe80::1",
                        "link-local": [
                            None
                        ]
                    }
                ]
            }
        }
    ]
}

# Instantiates the primary class
finder = JsonPath()

# Sets the data that is desired to be queried against
finder.set_data(json_data)

# Now we are interested in the path information where matches were found as well as the data
results: list[LocatedNode] = finder.find_located("$.items[*].address.'prefix-list'[*].prefix")

# Iterate through each found LocatedNode object
for data in results:

    # Print the normalized full path where the node was found
    print(f"{data.full_path}")
    # $['items'][0]['address']['prefix-list'][0]['prefix']

    # Iterate over the components of the found path
    # Returned elements will either be a 'str' for keys or 'int' for index values
    print(f"{', '.join([str(component) for component in data.path_components])}")
    # $, items, 0, adddress, prefix-list, 0, prefix

    # Access the found node. 
    print(f"{data.node}")
    # 2001:db8::1/64
```
