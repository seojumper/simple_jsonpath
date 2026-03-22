use pyo3::{
    exceptions,
    prelude::*,
    types::{PyBool, PyDict, PyDictMethods, PyFloat, PyList, PyListMethods, PyString},
};
use serde_json_path::NormalizedPath;
// use simple_jsonpath::SimpleJsonPath;

/// A Python module for querying JSON data using JSONPath expressions.
#[pymodule]
#[pyo3(name = "_simple_jsonpath")]
mod simple_jsonpath {
    use super::*;
    use pyo3::types::{PyBytes, PyInt, PySlice};
    use serde_json::Value;
    use serde_json_path::JsonPath;
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };
    use ustr::Ustr;

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
                inner: Arc::new(Mutex::new(HashMap::with_capacity(500))),
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
                pyresult.append((
                    Path::new(item.location()),
                    serialize_value(py, item.node())?,
                ))?;
            }

            Ok(pyresult)
        }
    }
    #[derive(Clone, Copy)]
    enum Index {
        U(Ustr),
        I(usize),
    }

    impl Index {
        fn to_normalized_segment(&self) -> String {
            match self {
                Self::I(num) => format!("[{num}]"),
                Self::U(u) => format!("['{}']", u.as_str()),
            }
        }
    }
    #[pyclass(sequence)]
    struct Path {
        indexes: Vec<Index>,
    }
    #[pymethods]
    impl Path {
        fn __getitem__<'py>(&self, index: Bound<'py, PyAny>) -> PyResult<Bound<'py, PyAny>> {
            let py = index.py();
            match index.cast::<PyInt>() {
                Ok(index) => {
                    let mut i = index.extract::<isize>()?;
                    if i < 0 {
                        if i == -1 {
                            let last = self.indexes.last().unwrap();
                            match last {
                                Index::U(u) => Ok(PyString::new(py, u.as_str()).into_any()),
                                Index::I(i) => Ok(PyInt::new(py, i).into_any()),
                            }
                        } else {
                            let abs_index = i.unsigned_abs();
                            if abs_index > self.__len__() {
                                Err(exceptions::PyIndexError::new_err("Index out of Range"))
                            } else if abs_index == self.__len__() {
                                Ok(PyString::new(py, "$").into_any())
                            } else {
                                let found =
                                    self.indexes.get(self.indexes.len() - abs_index).unwrap();
                                match found {
                                    Index::U(u) => Ok(PyString::new(py, u.as_str()).into_any()),
                                    Index::I(i) => Ok(PyInt::new(py, i).into_any()),
                                }
                            }
                        }
                    } else if i as usize == 0 {
                        Ok(PyString::new(py, "$").into_any())
                    } else {
                        i -= 1;
                        if let Some(i) = self.indexes.get(i as usize) {
                            match i {
                                Index::U(u) => Ok(PyString::new(py, u.as_str()).into_any()),
                                Index::I(i) => Ok(PyInt::new(py, i).into_any()),
                            }
                        } else {
                            Err(exceptions::PyIndexError::new_err("Index out of Range"))
                        }
                    }
                }
                Err(_) => match index.cast::<PySlice>() {
                    Ok(_) => Err(exceptions::PyValueError::new_err(
                        "Slicing operations are not supported",
                    )),
                    Err(e) => Err(e.into()),
                },
            }
        }
        fn __len__(&self) -> usize {
            self.indexes.len() + 1
        }
        fn __repr__(&self) -> String {
            if self.indexes.len() == 1 {
                "$".to_string()
            } else {
                let mut string = "$".to_string();
                string.extend(self.indexes[1..].iter().map(|i| match i {
                    Index::U(u) => format!("['{}']", u),
                    Index::I(num) => format!("[{num}]"),
                }));
                string
            }
        }
        fn __str__(&self) -> String {
            if self.indexes.len() == 1 {
                "$".to_string()
            } else {
                let mut string = "$".to_string();
                string.extend(self.indexes.iter().map(|i| i.to_normalized_segment()));
                string
            }
        }
        fn parent_path(&self) -> Option<Path> {
            if !self.indexes.is_empty() {
                let mut path = Vec::with_capacity(self.__len__() - 1);
                path.extend(
                    self.indexes[..self.indexes.len() - 1]
                        .iter()
                        .map(|index| *index),
                );
                Some(Path { indexes: path })
            } else {
                None
            }
        }
    }

    impl Path {
        fn new(location: &NormalizedPath) -> Self {
            let ids: Vec<Index> = location
                .iter()
                .map(|item| match item {
                    serde_json_path::PathElement::Name(name) => Index::U(Ustr::from(name)),
                    serde_json_path::PathElement::Index(num) => Index::I(*num),
                })
                .collect();
            Self { indexes: ids }
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
}

/// A struct to hold the located nodes for serialization.

/// Splits a normalized JSONPath into byte ranges `(start, end)` per component.
///
/// Example: `$['items'][0]['name']` -> `[(0,1), (1,10), (10,13), (13,21)]`

#[cfg(test)]
mod tests {
    // use super::simple_jsonpath::SimpleJsonPath;
    use rstest::rstest;
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

    // #[test]
    // fn split_normalized_path_into_component_ranges() {
    //     let data: Value = serde_json::from_str(r#"{"items":[{"name":"a"}] }"#).unwrap();
    //     let path = JsonPath::parse("$.items[0].name").unwrap();
    //     let located = path.query_located(&data).all();
    //     let location = located.first().unwrap().location();
    //     // $['items'][0]['name']
    //     println!("Getting here");
    //     let ranges = split_normalized_path_component_ranges(&location);
    //     println!("Getting here 2");
    //     let expected = vec![(0, 1), (3, 7), (0, 0), (15, 18)];
    //     for range in &ranges.1 {
    //         println!("Component: {}", &location.to_string()[range.0..range.1]);
    //     }
    //     println!("Getting here 3");

    //     assert_eq!(ranges.1, expected);
    // }
}
