# simple_jsonpath

## Installation

```bash
pip install simple_jsonpath
```

## About

This module is a JSONPath [RFC9535 - JSONPath: Query Expressions for JSON](https://datatracker.ietf.org/doc/html/rfc9535) utility library that supports performing querying for data in a JSON document.  It does **NOT** supporting modifying data
in place.

## Use

This module exposes a single simple type - a ***JsonPath*** which has two methods after instantiation.

- ***set_data()***: sets the data that will be queried against. If multiple queries will be performed against a single piece of JSON data, this helps with the type conversion cost involved. This function can be called with providing new data whenever the inner data held is wished to be changed while retaining already complied paths (useful for querying multiple similarly structured documents).
- ***find()***: given a path that is wished to be found in the previously set data, this function will perform the query logic. Mulitple calls to **find()** will query against the previously 'set' data.
- ***find_located()***: given a path that is wished to be found in the previously set data, this function can return a list of ***LocatedNode*** objects.  Each ***LocatedNode*** object will have attributes related to the path where the node was located as well as their corresponding data. This method is slower than ***find()***, so should ideally only be used when path information for the found nodes is needed.
- ***child()***: spawns a child instance of the ***JsonPath*** object that does not inherit its data, but maintains shared mutable access to the collection of
compiled paths.

## Child behavior

When the ***child()*** method is invoked, a child will be spawned from the current instance of the ***JsonPath*** object.

The child will not inherit the data from the parent, so a call to ***set_data()*** needa to be called on it for it to function.

It does however retain shared mutable access to the parent's collection of pre-parsed path objects which is shared across all spawned children.

This is useful for the pattern of:

1. Searching a document for a path query.

2. Then using those results returned as the basis of a new 'root element' for 'deeper' searches into a document.

Instead of assigning the query results to current instance, it can be beneficial to spawn a child for each result, and assign the result
data to the child or multiple children if more than one query result was returned.

With this pattern the 'base' parent object will automatically contain all parsed paths for the document that were searched by any descendant instance spawned from it, and children will have access to updates to the 'base' instance that any of their siblings make.

Then the 'base' parent object can be efficiently used on the next similarly structured document as all previously complied queries against the document are retained.

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

The same ***JsonPath*** object can then be reused with new data sets by calling ***set_data()*** on it again, and any previously parsed paths by the object will be retained.

Only when moving onto data of differing structure would it be potentially advisable to instantiate a new ***JsonPath*** object.

### 'Find Located' Example

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
    print(f"{data.path}")
    # $['items'][0]['address']['prefix-list'][0]['prefix']

    # Iterate over the components of the found path
    # Returned elements will either be a 'str' for keys or 'int' for index values
    print(f"{', '.join([str(component) for component in data.path])}")
    # $, items, 0, adddress, prefix-list, 0, prefix

    # Access the found node. 
    print(f"{data.node}")
    # 2001:db8::1/64
```

### 'Child' Example

The child pattern can be useful for speeding up processing of multiple similarly structured documents to avoid overhead
of parsing the same query strings many times.  Children are independent objects from the 'base' instancee, and children
can also spawn their own children.

```python
from simple_jsonpath import JsonPath


json_data_1 = {
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

json_data_2 = {
    "items": [
        {
            "address": {
                "prefix-list": [
                    {
                        "prefix": "2001:db8::2/64",
                        "eui-64": [
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
                        "prefix": "2001:db8::2/64",
                        "eui-64": [
                            None
                        ]
                    }
                ]
            }
        }
    ]
}

def process_document(data, finder: JsonPath)
    # The 'base' instance was instantiated outside of this fn below.

    # Sets the data that is desired to be queried against
    finder.set_data(json_data)

    # Search for interested data. This pattern will be cached in the base instance, which will
    # then be availble to all descendents of the base instance.
    results: list[Any] = finder.find("$.items[*]")

    # Iterate through each found result
    for data in results:
        # Spwn a child for each result
        child = finder.child()
        # Set the result data for the child
        child.set_data(data)

        # The first child that requests to find a pattern that has not yet been seen by the 'base' instance
        # will parse the pattern and insert it into the 'base' instance's cache of compiled patterns.
        #
        # The 'base' instance now has the pre-compiled pattern should it need to search for it.
        # 
        # All descendants of the 'base' instance now have access to the pre-compiled pattern to include
        # the child that will be spawned on the next iteration of this loop which will allow it to
        # process its own searches faster.
        results = child.find("$.address.'prefix-list'[*]")

        # .... further procesing....
all_documents = [json_data_1, json_data_2]

# create a single base JsonPath
finder = JsonPath()

for document in all_documents:
    # For each document that will be processed known to be similar in structure > pass in the same 'base' instance.
    #
    # By the time it has processed the first document (depending on how deep either iteself, or its child instnaces were able to traverse the document)
    # some/most/all of the possible paths that will need to be compiled have been. Which makes processing the next document in the series
    # quicker.
    process_document(document, finder)

```
