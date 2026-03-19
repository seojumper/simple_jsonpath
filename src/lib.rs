

use pyo3::prelude::*;
use serde_json_path::NormalizedPath;
use serde_json::Value;
use serde::{Serialize};


/// A Python module for querying JSON data using JSONPath expressions.
#[pymodule]
#[pyo3(name = "_simple_jsonpath")]
mod simple_jsonpath {
    use super::*;   
    use serde_json_path::JsonPath;
    use serde_json::Value;
    use std::{collections::HashMap, sync::Arc};

    /// A parser object that can be reused for multiple queries on the same JSON data.
    #[pyclass]
    pub struct SimpleJsonPath {
        inner: HashMap<String, JsonPath>,
        data: Arc<Value>,  
    }

    #[pymethods]
    impl SimpleJsonPath {
        /// Create a new SimpleJsonPath object with an empty cache and null data.
        #[new]
        pub fn new() -> PyResult<Self> {
            Ok(Self { inner: HashMap::new(), data: Arc::new(Value::Null) })
        }

        /// Set the JSON data for the parser from a JSON string.
        pub fn set_data_from_json_str(&mut self, input_data: &str) -> PyResult<()> {
            let value: Value = serde_json::from_str(input_data).map_err(|e| {
                pyo3::exceptions::PyValueError::new_err(format!("Invalid JSON string: {}", e))
            })?;
            self.data = Arc::new(value);
            Ok(())
        }

        /// Find the value(s) in the JSON data that match the given JSONPath expression, using a cache for parsed paths.
        pub fn find_from_set_data(&mut self, path: &str) -> PyResult<String> {
            if !self.inner.contains_key(path) {
                let json_path = JsonPath::parse(path).map_err(|e| {
                    pyo3::exceptions::PyValueError::new_err(format!(
                        "Parse error at position {} for path {}: {}",
                        e.position(),
                        path,
                        e.message()
                    ))
                })?;
                self.inner.insert(path.to_string(), json_path);
            }
            let json_path = self.inner.get(path).unwrap();
            let result = json_path.query(&self.data).all();
            let pyresult: String = serde_json::to_string(&result).map_err(|e| {
                    pyo3::exceptions::PyValueError::new_err(format!(
                        "Error converting result to JSON string: {}",
                        e
                    ))
                })?;
            Ok(pyresult)
        }

        /// Find the value(s) in the JSON data that match the given JSONPath expression, along with their locations, using a cache for parsed paths.
        pub fn find_location_from_set_data(
            &mut self,
            path: &str,
        ) -> PyResult<String> {
            if !self.inner.contains_key(path) {
                let json_path = JsonPath::parse(path).map_err(|e| {
                    pyo3::exceptions::PyValueError::new_err(format!(
                        "Parse error at position {} for path {}: {}",
                        e.position(),
                        path,
                        e.message()
                    ))
                })?;
                self.inner.insert(path.to_string(), json_path);
            }
            let json_path = self.inner.get(path).unwrap();
            let result = json_path.query_located(&self.data).all();
            let mut located_nodes = Vec::new();
            for item in &result {
                located_nodes.push(LocatedNode::new(item.location(), item.node()));
 
            }
            let pyresult: String = serde_json::to_string(&located_nodes).map_err(|e| {
                    pyo3::exceptions::PyValueError::new_err(format!(
                        "Error converting result to JSON string: {}",
                        e
                    ))
                })?;
            Ok(pyresult)
        }
    }
}


/// A struct to hold the located nodes for serialization.
#[derive(Serialize)]
struct LocatedNode<'a> {
    full_path: String,
    path_components: Vec<(usize, usize)>,
    node: &'a Value,
}

impl<'a> LocatedNode<'a> {
    fn new(full_path: &'a NormalizedPath, node: &'a Value) -> LocatedNode<'a> {
        let (full_path, path_components) = split_normalized_path_component_ranges(full_path);
        LocatedNode { full_path, path_components, node }
    }
}

/// Splits a normalized JSONPath into byte ranges `(start, end)` per component.
///
/// Example: `$['items'][0]['name']` -> `[(0,1), (1,10), (10,13), (13,21)]`
fn split_normalized_path_component_ranges(path: &NormalizedPath) -> (String, Vec<(usize, usize)>) {
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
            State::InQuotedField => {
                match c {
                    '\\' => state = State::InEscapedChar,
                    '\'' => {
                        match start {
                            Some(_) => 
                                ranges.push((start.take().unwrap(), i)),
                            None => ranges.push((i+1, i-1)),
                        }
                        state = State::InBracket;
                    },
                    _ => {
                        match start {
                            Some(_) => {},
                            None => start = Some(i),
                        }    
                    },
                }   
            }
            State::InEscapedChar => {
                state = State::InQuotedField;
            }
            State::InIndex => {
                if c == ']' {
                    if num_start.is_some() {
                        let num = chars[num_start.take().unwrap()..i].parse::<usize>().unwrap();
                        ranges.push((0,num));
                    }
                    state = State::Root;
                }
            }
        }
    }

    (path_str, ranges)
}

#[cfg(test)]
mod tests {
    use super::split_normalized_path_component_ranges;
    use super::simple_jsonpath::SimpleJsonPath;
    use rstest::rstest;
    use serde_json::Number;
    use serde_json::Value;
    use serde_json_path::{JsonPath};

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
            Ok(path) => {}
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



    // #[test]
    // fn find_location() {
    //     let data = r#"[
    //     {"name": "1/0/1", "description": "Test Interface 1/0/1", "media-type": "sfp", "switchport-conf": {"switchport": false}, "arp": {"timeout": 1000}, "bfd": {"Cisco-IOS-XE-bfd:enable": true, "Cisco-IOS-XE-bfd:local-address": "10.100.1.10", "Cisco-IOS-XE-bfd:interval-interface": {"msecs": 300, "min_rx": 300, "multiplier": 3}, "Cisco-IOS-XE-bfd:echo": false}, "bandwidth": {"kilobits": 1000000}, "mpls": {"Cisco-IOS-XE-mpls:ip": [null], "Cisco-IOS-XE-mpls:mtu": 1400}, "vrf": {"forwarding": "PRODUCTION"}, "ip": {"access-group": {"in": {"acl": {"acl-name": "ACL_IN", "in": [null]}}, "out": {"acl": {"acl-name": "ACL_OUT", "out": [null]}}}, "arp": {"inspection": {"limit": {"rate": 1000}, "trust": [null]}}, "address": {"primary": {"address": "10.100.1.10", "mask": "255.255.255.0"}}, "Cisco-IOS-XE-nat:nat": {"inside": [null]}, "helper-address": [{"address": "10.100.1.50", "vrf": "PRODUCTION"}], "pim": {"Cisco-IOS-XE-multicast:border": [null], "Cisco-IOS-XE-multicast:bfd": [null], "Cisco-IOS-XE-multicast:pim-mode-choice-cfg": {"sparse-mode": {}}, "Cisco-IOS-XE-multicast:dr-priority": 1000}, "proxy-arp": false, "redirects": false, "dhcp": {"Cisco-IOS-XE-dhcp:relay": {"information": {"option": {"vpn-id": [null]}}, "source-interface": "Loopback150"}}, "Cisco-IOS-XE-flow:flow": {"monitor-new": [{"name": "MONITOR_C9K_TEST", "direction": "input"}]}, "Cisco-IOS-XE-icmp:unreachables": false, "Cisco-IOS-XE-igmp:igmp": {"version": 3}, "Cisco-IOS-XE-nbar:nbar": {"protocol-discovery": {}}, "Cisco-IOS-XE-ospf:router-ospf": {"ospf": {"process-id": [{"id": 10, "area": [{"area-id": 0}]}], "cost": 10, "dead-interval": 60, "hello-interval": 15, "mtu-ignore": true, "message-digest-key": [{"id": 1, "md5": {"auth-type": 0, "auth-key": "cisco"}}], "network": {"point-to-point": [null]}, "multi-area": {"multi-area-id": [{"area-id": 1}, {"area-id": 2}]}, "priority": 10, "ttl-security": {"hops": 5}}}}, "ipv6": {"address": {"prefix-list": [{"prefix": "2001:db8::1/64", "eui-64": [null]}], "link-local-address": [{"address": "fe80::1", "link-local": [null]}]}, "enable": [null], "mtu": 1600, "nd": {"Cisco-IOS-XE-nd:ra": {"suppress": {"all": [null]}}}, "Cisco-IOS-XE-flow:flow": {"monitor-new": [{"name": "MONITOR_C9K_TEST_V6", "direction": "input"}]}, "Cisco-IOS-XE-multicast:pim-conf": {"pim": true}, "Cisco-IOS-XE-multicast:pim-container": {"bfd": [null], "dr-priority": 1000}}, "load-interval": 30, "logging": {"event": {"link-status-enable": true}}, "mtu": 1600, "Cisco-IOS-XE-cdp:cdp": {"enable": true, "tlv": {"default-wrp": {"app": false}, "server-location-config": false, "location-config": false}}, "Cisco-IOS-XE-dot1x:dot1x": {"pae": "authenticator", "max-reauth-req": 3, "max-req": 3, "timeout": {"auth-period": 10000, "held-period": 40000, "quiet-period": 10000, "ratelimit-period": 5000, "server-timeout": 5000, "start-period": 2000, "supp-timeout": 2000, "tx-period": 2000}}, "Cisco-IOS-XE-ethernet:port-settings": {"speed": {"speed-value": "100"}, "auto-negotiation": "disable"}, "Cisco-IOS-XE-ethernet:speed": {"nonegotiate": [null]}, "Cisco-IOS-XE-ospfv3:ospfv3": {"cost-config": {"value": 10}, "network-type": {"point-to-point": [null]}}, "Cisco-IOS-XE-policy:service-policy": {"input": "POLICY_IN", "output": "POLICY_OUT"}, "Cisco-IOS-XE-sanet:authentication": {"periodic": [null], "timer": {"reauthenticate": {"server-config": [null]}}}, "Cisco-IOS-XE-sanet:mab": {"eap": [null]}, "Cisco-IOS-XE-snmp:snmp": {"trap": {"link-status": true}}},
    //     {"name": "1/0/1", "description": "Test Interface 1/0/1", "media-type": "sfp", "switchport-conf": {"switchport": false}, "arp": {"timeout": 1000}, "bfd": {"Cisco-IOS-XE-bfd:enable": true, "Cisco-IOS-XE-bfd:local-address": "10.100.1.10", "Cisco-IOS-XE-bfd:interval-interface": {"msecs": 300, "min_rx": 300, "multiplier": 3}, "Cisco-IOS-XE-bfd:echo": false}, "bandwidth": {"kilobits": 1000000}, "mpls": {"Cisco-IOS-XE-mpls:ip": [null], "Cisco-IOS-XE-mpls:mtu": 1400}, "vrf": {"forwarding": "PRODUCTION"}, "ip": {"access-group": {"in": {"acl": {"acl-name": "ACL_IN", "in": [null]}}, "out": {"acl": {"acl-name": "ACL_OUT", "out": [null]}}}, "arp": {"inspection": {"limit": {"rate": 1000}, "trust": [null]}}, "address": {"primary": {"address": "10.100.1.10", "mask": "255.255.255.0"}}, "Cisco-IOS-XE-nat:nat": {"inside": [null]}, "helper-address": [{"address": "10.100.1.50", "vrf": "PRODUCTION"}], "pim": {"Cisco-IOS-XE-multicast:border": [null], "Cisco-IOS-XE-multicast:bfd": [null], "Cisco-IOS-XE-multicast:pim-mode-choice-cfg": {"sparse-mode": {}}, "Cisco-IOS-XE-multicast:dr-priority": 1000}, "proxy-arp": false, "redirects": false, "dhcp": {"Cisco-IOS-XE-dhcp:relay": {"information": {"option": {"vpn-id": [null]}}, "source-interface": "Loopback150"}}, "Cisco-IOS-XE-flow:flow": {"monitor-new": [{"name": "MONITOR_C9K_TEST", "direction": "input"}]}, "Cisco-IOS-XE-icmp:unreachables": false, "Cisco-IOS-XE-igmp:igmp": {"version": 3}, "Cisco-IOS-XE-nbar:nbar": {"protocol-discovery": {}}, "Cisco-IOS-XE-ospf:router-ospf": {"ospf": {"process-id": [{"id": 10, "area": [{"area-id": 0}]}], "cost": 10, "dead-interval": 60, "hello-interval": 15, "mtu-ignore": true, "message-digest-key": [{"id": 1, "md5": {"auth-type": 0, "auth-key": "cisco"}}], "network": {"point-to-point": [null]}, "multi-area": {"multi-area-id": [{"area-id": 1}, {"area-id": 2}]}, "priority": 10, "ttl-security": {"hops": 5}}}}, "ipv6": {"address": {"prefix-list": [{"prefix": "2001:db8::1/64", "eui-64": [null]}], "link-local-address": [{"address": "fe80::1", "link-local": [null]}]}, "enable": [null], "mtu": 1600, "nd": {"Cisco-IOS-XE-nd:ra": {"suppress": {"all": [null]}}}, "Cisco-IOS-XE-flow:flow": {"monitor-new": [{"name": "MONITOR_C9K_TEST_V6", "direction": "input"}]}, "Cisco-IOS-XE-multicast:pim-conf": {"pim": true}, "Cisco-IOS-XE-multicast:pim-container": {"bfd": [null], "dr-priority": 1000}}, "load-interval": 30, "logging": {"event": {"link-status-enable": true}}, "mtu": 1600, "Cisco-IOS-XE-cdp:cdp": {"enable": true, "tlv": {"default-wrp": {"app": false}, "server-location-config": false, "location-config": false}}, "Cisco-IOS-XE-dot1x:dot1x": {"pae": "authenticator", "max-reauth-req": 3, "max-req": 3, "timeout": {"auth-period": 10000, "held-period": 40000, "quiet-period": 10000, "ratelimit-period": 5000, "server-timeout": 5000, "start-period": 2000, "supp-timeout": 2000, "tx-period": 2000}}, "Cisco-IOS-XE-ethernet:port-settings": {"speed": {"speed-value": "100"}, "auto-negotiation": "disable"}, "Cisco-IOS-XE-ethernet:speed": {"nonegotiate": [null]}, "Cisco-IOS-XE-ospfv3:ospfv3": {"cost-config": {"value": 10}, "network-type": {"point-to-point": [null]}}, "Cisco-IOS-XE-policy:service-policy": {"input": "POLICY_IN", "output": "POLICY_OUT"}, "Cisco-IOS-XE-sanet:authentication": {"periodic": [null], "timer": {"reauthenticate": {"server-config": [null]}}}, "Cisco-IOS-XE-sanet:mab": {"eap": [null]}, "Cisco-IOS-XE-snmp:snmp": {"trap": {"link-status": true}}}
    //     ]"#;
    //     let path = "$[*].name";
    //     let data: Value = serde_json::from_str(data).unwrap();
    //     let data_str = serde_json::to_string(&data).unwrap();
    //     let mut finder = SimpleJsonPath::new().unwrap();
    //     finder.set_data_from_json_str(&data_str).unwrap();
    //     let result: Value = serde_json::from_str(&finder.find_location_from_set_data(path).unwrap()).unwrap();
    //     assert_eq!(result.as_array().unwrap().len(), 2);
    //     let array = result.as_array().unwrap();
    //     let path_components_1 = Value::Array(vec![Value::String("$".to_string()), Value::Number(Number::from_str("0").unwrap()), Value::String("name".to_string())]);
    //     let path_components_2 = Value::Array(vec![Value::String("$".to_string()), Value::Number(Number::from_str("1").unwrap()), Value::String("name".to_string())]);
    //     assert_eq!(array.len(), 2);

    //     println!("{:?}", array);
    //     let located_node1 = array[0].as_object().unwrap();
    //     let full_path = located_node1.get("full_path").unwrap().as_str().unwrap();
    //     assert_eq!(full_path, " ");
    //     let path_components = located_node1.get("path_components").unwrap().as_array().unwrap();
    //     assert_eq!(path_components, path_components_1.as_array().unwrap());
    //     let nodes = located_node1.get("nodes").unwrap().as_array().unwrap();
    //     assert_eq!(nodes.len(), 1);


    //     let located_node2 = array[1].as_object().unwrap();
    //     let full_path = located_node2.get("full_path").unwrap().as_str().unwrap();
    //     assert_eq!(full_path, "$[1]['name']");
    //     let path_components = located_node2.get("path_components").unwrap().as_array().unwrap();
    //     assert_eq!(path_components, path_components_2.as_array().unwrap());
    //     let nodes = located_node2.get("nodes").unwrap().as_array().unwrap();
    //     assert_eq!(nodes.len(), 1);
        



    //     // assert_eq!(result.as_array().unwrap()[0].as_str().unwrap(), "1/0/1");
    // }
}
