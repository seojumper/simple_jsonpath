use pyo3::{prelude::*};
use serde_json_path::{JsonPath};

/// A Python module for querying JSON data using JSONPath expressions.
#[pymodule]
mod simple_jsonpath {
    use super::*;
    
    /// Find the value(s) in the JSON data that match the given JSONPath expression.
    #[pyfunction]
    fn find(path: String, data: String) -> PyResult<Option<String>> {
        match serde_json::from_str(&data) {
            Ok(json) => {
                match JsonPath::parse(&path) {
                    Ok(json_path) => {
                        let result = json_path.query(&json).all();
                        if result.is_empty() {
                            Ok(None)
                        } else {
                            // Convert the result to a JSON string
                            match serde_json::to_string(&result) {
                                Ok(json_result) => Ok(Some(json_result)),
                                Err(e) => return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Error converting result to JSON string: {}", e))),
                            }
                        }
                    }
                    Err(e) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Parse error at position {} for path {}:\n{}", e.position(), path, e.message()))),
                }
            }
            Err(e) => Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!("Invalid JSON data: {}", e))),
        }
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;
    use serde_json_path::{JsonPath, ParseError};
    use serde_json::Value;

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
    fn find() {
        let data = r#"
        {"name": "1/0/1", "description": "Test Interface 1/0/1", "media-type": "sfp", "switchport-conf": {"switchport": false}, "arp": {"timeout": 1000}, "bfd": {"Cisco-IOS-XE-bfd:enable": true, "Cisco-IOS-XE-bfd:local-address": "10.100.1.10", "Cisco-IOS-XE-bfd:interval-interface": {"msecs": 300, "min_rx": 300, "multiplier": 3}, "Cisco-IOS-XE-bfd:echo": false}, "bandwidth": {"kilobits": 1000000}, "mpls": {"Cisco-IOS-XE-mpls:ip": [null], "Cisco-IOS-XE-mpls:mtu": 1400}, "vrf": {"forwarding": "PRODUCTION"}, "ip": {"access-group": {"in": {"acl": {"acl-name": "ACL_IN", "in": [null]}}, "out": {"acl": {"acl-name": "ACL_OUT", "out": [null]}}}, "arp": {"inspection": {"limit": {"rate": 1000}, "trust": [null]}}, "address": {"primary": {"address": "10.100.1.10", "mask": "255.255.255.0"}}, "Cisco-IOS-XE-nat:nat": {"inside": [null]}, "helper-address": [{"address": "10.100.1.50", "vrf": "PRODUCTION"}], "pim": {"Cisco-IOS-XE-multicast:border": [null], "Cisco-IOS-XE-multicast:bfd": [null], "Cisco-IOS-XE-multicast:pim-mode-choice-cfg": {"sparse-mode": {}}, "Cisco-IOS-XE-multicast:dr-priority": 1000}, "proxy-arp": false, "redirects": false, "dhcp": {"Cisco-IOS-XE-dhcp:relay": {"information": {"option": {"vpn-id": [null]}}, "source-interface": "Loopback150"}}, "Cisco-IOS-XE-flow:flow": {"monitor-new": [{"name": "MONITOR_C9K_TEST", "direction": "input"}]}, "Cisco-IOS-XE-icmp:unreachables": false, "Cisco-IOS-XE-igmp:igmp": {"version": 3}, "Cisco-IOS-XE-nbar:nbar": {"protocol-discovery": {}}, "Cisco-IOS-XE-ospf:router-ospf": {"ospf": {"process-id": [{"id": 10, "area": [{"area-id": 0}]}], "cost": 10, "dead-interval": 60, "hello-interval": 15, "mtu-ignore": true, "message-digest-key": [{"id": 1, "md5": {"auth-type": 0, "auth-key": "cisco"}}], "network": {"point-to-point": [null]}, "multi-area": {"multi-area-id": [{"area-id": 1}, {"area-id": 2}]}, "priority": 10, "ttl-security": {"hops": 5}}}}, "ipv6": {"address": {"prefix-list": [{"prefix": "2001:db8::1/64", "eui-64": [null]}], "link-local-address": [{"address": "fe80::1", "link-local": [null]}]}, "enable": [null], "mtu": 1600, "nd": {"Cisco-IOS-XE-nd:ra": {"suppress": {"all": [null]}}}, "Cisco-IOS-XE-flow:flow": {"monitor-new": [{"name": "MONITOR_C9K_TEST_V6", "direction": "input"}]}, "Cisco-IOS-XE-multicast:pim-conf": {"pim": true}, "Cisco-IOS-XE-multicast:pim-container": {"bfd": [null], "dr-priority": 1000}}, "load-interval": 30, "logging": {"event": {"link-status-enable": true}}, "mtu": 1600, "Cisco-IOS-XE-cdp:cdp": {"enable": true, "tlv": {"default-wrp": {"app": false}, "server-location-config": false, "location-config": false}}, "Cisco-IOS-XE-dot1x:dot1x": {"pae": "authenticator", "max-reauth-req": 3, "max-req": 3, "timeout": {"auth-period": 10000, "held-period": 40000, "quiet-period": 10000, "ratelimit-period": 5000, "server-timeout": 5000, "start-period": 2000, "supp-timeout": 2000, "tx-period": 2000}}, "Cisco-IOS-XE-ethernet:port-settings": {"speed": {"speed-value": "100"}, "auto-negotiation": "disable"}, "Cisco-IOS-XE-ethernet:speed": {"nonegotiate": [null]}, "Cisco-IOS-XE-ospfv3:ospfv3": {"cost-config": {"value": 10}, "network-type": {"point-to-point": [null]}}, "Cisco-IOS-XE-policy:service-policy": {"input": "POLICY_IN", "output": "POLICY_OUT"}, "Cisco-IOS-XE-sanet:authentication": {"periodic": [null], "timer": {"reauthenticate": {"server-config": [null]}}}, "Cisco-IOS-XE-sanet:mab": {"eap": [null]}, "Cisco-IOS-XE-snmp:snmp": {"trap": {"link-status": true}}}"#;
        let path = "$.name";
        let data: Value = serde_json::from_str(data).unwrap();
        let result = JsonPath::parse(path).unwrap().query(&data).all();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].as_str().unwrap(), "1/0/1");
    }
}
