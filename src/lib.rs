use cache::CacheOption;



pub mod parser;
pub mod annotations;
pub mod schema;
pub mod registry;
pub mod cache;
pub mod spec;
pub mod device;
pub mod watch;
use crate::registry::RegistryCache;
use crate::registry::RegistrySpecDB;

pub fn cdi_list_vendors() {
	let options: Vec<Box<dyn CacheOption>> = vec![
		Box::new(cache::WithAutoRefresh(true)), 
	];
	let registry = registry::get_registry(options);
	let vendors = registry.spec_db().list_vendors();
	
	if vendors.is_empty() {
		println!("No CDI vendors found");
		return;
	}
	println!("CDI vendors found");
	for (idx, vendor) in vendors.iter().enumerate() {
		println!(" {}. {} ({} CDI Spec Files)", idx, vendor, registry.spec_db().get_vendor_specs(vendor).len());
	}
}




#[cfg(test)]
mod tests {
	

}
