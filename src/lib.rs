use pyo3::{
    exceptions,
    prelude::*,
    types::{PyBool, PyDict, PyDictMethods, PyFloat, PyInt, PyList, PyListMethods, PyString},
};
use serde_json_path::NormalizedPath;
// use simple_jsonpath::SimpleJsonPath;

/// A Python module for querying JSON data using JSONPath expressions.
#[pymodule]
#[pyo3(name = "_simple_jsonpath")]
mod simple_jsonpath {
    use super::*;
    use pyo3::types::PyBytes;
    use serde_json::Value;
    use serde_json_path::JsonPath;
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };

    /// A parser object that can be reused for multiple queries on the same JSON data.
    #[pyclass]
    pub struct SimpleJsonPath {
        pub inner: Arc<Mutex<HashMap<String, JsonPath>>>,
        pub data: Option<Value>,
    }

    #[pymethods]
    impl SimpleJsonPath {
        /// Create a new SimpleJsonPath object with an empty cache and null data.
        #[new]
        pub fn new() -> PyResult<Self> {
            Ok(Self {
                inner: Arc::new(Mutex::new(HashMap::new())),
                data: None,
            })
        }

        /// Creates a child instance of a SimpleJsonPath from an existing one.
        ///
        /// The child instance will have mutable access to the parent's
        /// collection of pre-compiled path objects.
        ///
        /// This allows for nested series of searches within a single document
        /// to have all of its pre-compiled paths represented by a single
        /// object > that single object used to parse the next document of
        /// similar structure.
        pub fn child(&self) -> PyResult<Self> {
            Ok(Self {
                inner: self.inner.clone(),
                data: None,
            })
        }

        pub fn has_data(&self) -> bool {
            self.data.is_some()
        }

        /// Set the JSON data for the parser from a JSON string.
        pub fn set_data<'py>(&mut self, input_data: Bound<'py, PyBytes>) -> PyResult<()> {
            let value: Value = serde_json::from_slice(input_data.as_bytes()).map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("Invalid JSON string: {}", e))
            })?;
            self.data = Some(value);
            Ok(())
        }

        /// Find the value(s) in the JSON data that match the given JSONPath expression, using a cache for parsed paths.
        pub fn find<'b>(&mut self, path: Bound<'b, PyString>) -> PyResult<Bound<'b, PyList>> {
            let py = path.py();
            let path = path.to_string();
            let result = {
                let mut lock = match self.inner.lock() {
                    Ok(guard) => guard,
                    Err(e) => e.into_inner(),
                };
                let json_path = lock.entry(path.clone()).or_insert({
                    JsonPath::parse(&path).map_err(|e| {
                        pyo3::exceptions::PyValueError::new_err(format!(
                            "Parse error at position {} for path {}: {}",
                            e.position(),
                            path,
                            e.message()
                        ))
                    })?
                });
                match &self.data {
                    Some(data) => json_path.query(data).all(),
                    None => Err(pyo3::exceptions::PyValueError::new_err(
                        "JsonPath object has not been provided data through the 'set_data()' method.",
                    ))?,
                }
            };
            let values = PyList::empty(py);
            for value in result {
                values.append(serialize_value(py, value)?)?
            }
            Ok(values)
        }

        /// Find the value(s) in the JSON data that match the given JSONPath expression, along with their locations, using a cache for parsed paths.
        pub fn find_located<'py>(
            &mut self,
            path: Bound<'py, PyString>,
        ) -> PyResult<Bound<'py, PyList>> {
            let py = path.py();
            let path = path.to_str()?;
            let result = {
                let mut lock = match self.inner.lock() {
                    Ok(guard) => guard,
                    Err(e) => e.into_inner(),
                };
                let json_path = lock.entry(path.to_string()).or_insert({
                    JsonPath::parse(path).map_err(|e| {
                        pyo3::exceptions::PyValueError::new_err(format!(
                            "Parse error at position {} for path {}: {}",
                            e.position(),
                            path,
                            e.message()
                        ))
                    })?
                });
                match &self.data {
                    Some(data) => json_path.query_located(data).all(),
                    None => Err(pyo3::exceptions::PyValueError::new_err(
                        "JsonPath object has not been provided data through the 'set_data()' method.",
                    ))?,
                }
            };
            let pyresult = PyList::empty(py);
            for item in &result {
                pyresult.append(
                    LocatedNode::new(item.location(), item.node()).convert_to_py_any(py)?,
                )?;
            }

            Ok(pyresult)
        }
    }
    fn serialize_value<'a>(py: Python<'a>, value: &'a Value) -> PyResult<Bound<'a, PyAny>> {
        match value {
            Value::Null => Ok(py.None().into_bound(py)),
            Value::Bool(b) => Ok(PyBool::new(py, *b).to_owned().into_any()),
            Value::Number(number) => match number.as_i64() {
                Some(num) => Ok(PyInt::new(py, num).into_any()),
                None => match number.as_f64() {
                    Some(num) => Ok(PyFloat::new(py, num).into_any()),
                    None => Err(exceptions::PyValueError::new_err("Number too large")),
                },
            },
            Value::String(string) => Ok(PyString::new(py, string).into_any()),
            Value::Array(values) => {
                let list = PyList::empty(py);
                for value in values {
                    list.append(serialize_value(py, value)?)?;
                }
                Ok(list.into_any())
            }
            Value::Object(map) => {
                let dict = PyDict::new(py);
                let dict = dict.into_mapping();
                for (k, v) in map.iter() {
                    dict.set_item(k, serialize_value(py, v)?)?
                }
                Ok(dict.into_any())
            }
        }
    }

    struct LocatedNode<'a> {
        full_path: String,
        path_components: Vec<(usize, usize)>,
        node: &'a Value,
    }

    impl<'a> LocatedNode<'a> {
        fn new(full_path: &NormalizedPath, node: &'a Value) -> Self {
            let (full_path, path_components) = split_normalized_path_component_ranges(full_path);
            LocatedNode {
                full_path,
                path_components,
                node,
            }
        }
        // Implement the conversion function
        fn convert_to_py_any<'py>(self, py: Python<'py>) -> PyResult<Bound<'py, PyAny>> {
            // Convert the inner value to a Python integer;
            let dict = PyDict::new(py);
            let mapping = dict.as_mapping();
            mapping.set_item("full_path", self.full_path)?;
            mapping.set_item("path_components", self.path_components)?;
            mapping.set_item("node", serialize_value(py, self.node)?)?;

            Ok(dict.into_any())
        }
    }

    pub fn split_normalized_path_component_ranges(
        path: &NormalizedPath,
    ) -> (String, Vec<(usize, usize)>) {
        let path_str = path.to_string();
        let chars = &path_str.as_str();

        let mut ranges = Vec::with_capacity(path.len() + 1);
        ranges.push((0, 1)); // Start with the root component '$'

        #[derive(Debug)]
        enum State {
            Start,
            Root,
            InBracket,
            InQuotedField,
            InEscapedChar,
            InIndex,
        }
        let mut num_start = None;
        let mut start = None;
        let mut state = State::Start;
        for (i, c) in chars.char_indices() {
            match state {
                State::Start => {
                    if c == '$' {
                        state = State::Root;
                    }
                }
                State::Root => {
                    if c == '[' {
                        state = State::InBracket;
                    }
                }
                State::InBracket => {
                    if c == ']' {
                        state = State::Root;
                    } else if c == '\'' {
                        state = State::InQuotedField;
                    } else {
                        state = State::InIndex;
                        num_start = Some(i);
                    }
                }
                State::InQuotedField => match c {
                    '\\' => state = State::InEscapedChar,
                    '\'' => {
                        match start {
                            Some(_) => ranges.push((start.take().unwrap(), i)),
                            None => ranges.push((i + 1, i - 1)),
                        }
                        state = State::InBracket;
                    }
                    _ => match start {
                        Some(_) => {}
                        None => start = Some(i),
                    },
                },
                State::InEscapedChar => {
                    state = State::InQuotedField;
                }
                State::InIndex => {
                    if c == ']' {
                        if num_start.is_some() {
                            let num = chars[num_start.take().unwrap()..i]
                                .parse::<usize>()
                                .unwrap();
                            ranges.push((0, num));
                        }
                        state = State::Root;
                    }
                }
            }
        }

        (path_str, ranges)
    }
}

/// A struct to hold the located nodes for serialization.

/// Splits a normalized JSONPath into byte ranges `(start, end)` per component.
///
/// Example: `$['items'][0]['name']` -> `[(0,1), (1,10), (10,13), (13,21)]`

#[cfg(test)]
mod tests {
    // use super::simple_jsonpath::SimpleJsonPath;
    use super::simple_jsonpath::split_normalized_path_component_ranges;
    use rstest::rstest;
    use serde_json::Value;
    use serde_json_path::JsonPath;

    #[rstest]
    #[case("$.router.bgp[0].bgp.router_id.interface")]
    #[case("$.router.'Cisco-IOS-XE-bgp:bgp'[0].bgp.default.'ipv4-unicast'")]
    #[case("$.router.'Cisco-IOS-XE-bgp:bgp'[0].bgp.'log-neighbor-changes'")]
    #[case("$.router.'Cisco-IOS-XE-bgp:bgp'[0].bgp.'graceful-restart'")]
    #[case("$.router.'Cisco-IOS-XE-bgp:bgp'[0].bgp.'update-delay'")]
    #[case("$.router.'Cisco-IOS-XE-bgp:bgp'[0].template.'peer-policy'[*]")]
    #[case(
        "$.router.'Cisco-IOS-XE-bgp:bgp'[0].'address-family'.'no-vrf'.ipv4[0].'ipv4-unicast'.'aggregate-address'[*]"
    )]
    fn path(#[case] input: &str) {
        let result = JsonPath::parse(input);
        match &result {
            Ok(_) => {}
            Err(e) => {
                println!("Parse error at position {}: {}", e.position(), e.message());
            }
        }
        assert!(result.is_ok());
    }

    #[test]
    fn split_normalized_path_into_component_ranges() {
        let data: Value = serde_json::from_str(r#"{"items":[{"name":"a"}] }"#).unwrap();
        let path = JsonPath::parse("$.items[0].name").unwrap();
        let located = path.query_located(&data).all();
        let location = located.first().unwrap().location();
        // $['items'][0]['name']
        println!("Getting here");
        let ranges = split_normalized_path_component_ranges(&location);
        println!("Getting here 2");
        let expected = vec![(0, 1), (3, 7), (0, 0), (15, 18)];
        for range in &ranges.1 {
            println!("Component: {}", &location.to_string()[range.0..range.1]);
        }
        println!("Getting here 3");

        assert_eq!(ranges.1, expected);
    }
}
