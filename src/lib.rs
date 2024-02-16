

mod parser;
mod cdi;


#[cfg(test)]


mod tests {
	


	use std::collections::HashMap;

	use crate::cdi::annotations;
	use crate::parser;

	
	#[test]

	fn qualified_name() {
		let vendor = "nvidia.com";
		let class = "gpu";
		let name = "0";
		let device = parser::qualified_name(vendor, class, name);
		assert_eq!(device, "nvidia.com/gpu=0");
		assert_eq!(parser::is_qualified_name(&device), true);
	}

	#[test]
	fn parse_qualified_name() {
		let device = "nvidia.com/gpu=0";
		match parser::parse_qualified_name(device) {
			Ok((vendor, class, name)) => {
				assert_eq!(vendor, "nvidia.com");
				assert_eq!(class, "gpu");
				assert_eq!(name, "0");
			},
			Err(e) => {
				println!("error: {}", e);
			}
		}
	}

	#[test]
	fn parse_device() {
		let device = "nvidia.com/gpu=0";
		let (vendor, class, name) = parser::parse_device(device);
		assert_eq!(vendor, "nvidia.com");
		assert_eq!(class, "gpu");
		assert_eq!(name, "0");
	}

	#[test]
	fn parse_qualifier() {
		let qualifier = "nvidia.com/gpu";
		let (vendor, class) = parser::parse_qualifier(qualifier);
		assert_eq!(vendor, "nvidia.com");
		assert_eq!(class, "gpu");
	}

	#[test] 
	fn validate_vendor_name() {
		let vendor = "nvidia.com";
		assert!(parser::validate_vendor_name(vendor).is_ok());

		let vendor = "nvi((dia";
		assert!(parser::validate_vendor_name(vendor).is_err());
	}
	#[test]
	fn validate_class_name() {
		let class = "gpu";
		assert!(parser::validate_class_name(class).is_ok());

		let class = "g(pu";
		assert!(parser::validate_class_name(class).is_err());
	}

	#[test] 
	fn validate_device_name() {
		let name = "0";
		assert!(parser::validate_device_name(name).is_ok());

		let name = "0(";
		assert!(parser::validate_device_name(name).is_err());
	}
	#[test]
	fn validate_vendor_or_class_name() {
		let name = "nvidia.com";
		assert!(parser::validate_vendor_or_class_name(name).is_ok());

		let name = "nvi((dia.com";
		assert!(parser::validate_vendor_or_class_name(name).is_err());
	}

	#[test]
	fn parse_annotations() {

		
		let mut cdi_devices = HashMap::new();

		cdi_devices.insert("cdi.k8s.io/vfio17".to_string(), "nvidia.com/gpu=0".to_string());
		cdi_devices.insert("cdi.k8s.io/vfio18".to_string(), "nvidia.com/gpu=1".to_string());
		cdi_devices.insert("cdi.k8s.io/vfio19".to_string(), "nvidia.com/gpu=all".to_string());

		match annotations::parse_annotations(cdi_devices) {
			Ok((keys, devices)) => {
				assert_eq!(keys.len(), 3);
				assert_eq!(devices.len(), 3);
			},
			Err(e) => {
				println!("error: {}", e);
			}  
		} 


	}
	
	#[test]
	fn annotation_value() {
		let devices = vec!["nvidia.com/gpu=0".to_string(), "nvidia.com/gpu=1".to_string()];
		match annotations::annotation_value(devices) {
			Ok(value) => {
				assert_eq!(value, "nvidia.com/gpu=0,nvidia.com/gpu=1");
			},
			Err(e) => {
				println!("error: {}", e);
			}
		}
	}

	#[test]
	fn annotation_key() {
		let plugin_name = "nvida-device-plugin";
		let device_id = "gpu=0";
		match annotations::annotation_key(plugin_name, device_id) {
			Ok(key) => {
				assert_eq!(key, "nvidia-device-plugin_gpu=0");
			},
			Err(e) => {
				println!("error: {}", e);
			}
		}
	}


}
