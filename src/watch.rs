
use notify::{Watcher, watcher, RecursiveMode};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::error::Error;
use std::thread;
use std::time::Duration;
use std::sync::mpsc::channel;
use notify::DebouncedEvent;
use anyhow::anyhow;
pub struct Watch {
	watcher: Arc<Mutex<notify::RecommendedWatcher>>,
	tracked: Arc<Mutex<HashMap<String, bool>>>,
}


impl Watch {
	pub fn new() -> Watch {
		Watch {
			watcher: Arc::new(Mutex::new(notify::watcher(channel().0, Duration::from_secs(2)).unwrap())),
			tracked: Arc::new(Mutex::new(HashMap::new())),
		}
	}

	pub fn setup(&mut self, dirs: Vec<String>, dir_errors: &mut HashMap<String, Box<dyn Error + Send + Sync + 'static>>) {
		let mut tracked = HashMap::new();
		for dir in &dirs {
		    tracked.insert(dir.clone(), false);
		}
		self.tracked = Arc::new(Mutex::new(tracked));
	
		let (tx, rx) = std::sync::mpsc::channel();
		match watcher(tx, Duration::from_secs(2)) {
		    Ok(mut watch) => {
			for dir in dirs.iter() {
			    if let Err(e) = watch.watch(dir, RecursiveMode::Recursive) {
				dir_errors.insert(dir.clone(), Box::new(e));
			    } else {
				self.tracked.lock().unwrap().insert(dir.clone(), true);
			    }
			}
			self.watcher = Arc::new(Mutex::new(watch));
		    },
		    Err(e) => {
			for dir in dirs {
			    dir_errors.insert(dir, Box::new(e));
			}
		    },
		}
		self.update(dir_errors, Vec::new());
	}

	fn start(&self, refresh: impl Fn() -> Result<(), Box<dyn std::error::Error>> + Send + 'static + Clone, dir_errors: &mut HashMap<String,  Box<dyn std::error::Error + Send + Sync + 'static>>) {
		let refresh_clone = refresh.clone();
	
		thread::spawn(move || {
		    // Assuming `watch` is adapted to be callable in this context.
		    // You might need to pass additional parameters or clone other necessary data.
		    self.watch(refresh_clone, dir_errors);
		});
	}
	pub fn stop(&self) {
		/*
		let mut watcher = match self.watcher.lock() {
		    Ok(guard) => guard,
		    Err(poisoned) => poisoned.into_inner(),
		};
	 	*/
		let mut watcher = self.watcher.lock().unwrap();
		let mut tracked = self.tracked.lock().unwrap();
	 

		for (dir, _) in tracked.iter() {
		    if let Err(e) = watcher.unwatch(dir) {
			println!("Error stopping watcher: {:?}", e);
		    }
		}
		tracked.clear();
	}

	fn watch(&self, refresh: impl Fn() -> Result<(), Box<dyn std::error::Error>> + Send + 'static, dir_errors:  &mut HashMap<String,  Box<dyn std::error::Error + Send + Sync + 'static>>) {
	    let (tx, rx) = channel();
	    let mut watcher = watcher(tx, Duration::from_secs(10)).unwrap();
	    
	    // Assuming you've already added directories to watch somewhere
	    // for dir in self.tracked.lock().unwrap().keys() {
	    //     watcher.watch(dir, RecursiveMode::Recursive).unwrap();
	    // }
    
	    loop {
		match rx.recv() {
		    Ok(event) => match event {
			DebouncedEvent::Write(path) | DebouncedEvent::Remove(path) | DebouncedEvent::Rename(_, path) => {
			    if path.extension().map_or(true, |ext| ext != "json" && ext != "yaml") {
				continue;
			    }
    
			    let mut tracked = self.tracked.lock().unwrap();
			    let file_name = path.to_str().unwrap_or_default().to_string();
    
			    if let DebouncedEvent::Remove(_) = event {
				if *tracked.get(&file_name).unwrap_or(&false) {
				    self.update(dir_errors, vec![file_name]);
				} else {
				    self.update(dir_errors, Vec::new());
				}
			    }
			    refresh().unwrap(); // Handle error as needed
			},
			_ => continue,
		    },
		    Err(_) => break,
		}
	    }
	}

	pub fn update(&self, dir_errors: &mut HashMap<String,  Box<dyn std::error::Error + Send + Sync + 'static>>, removed: Vec<String>) -> bool {
		let mut update = false;
		let mut watcher = self.watcher.lock().unwrap();
		let mut tracked = self.tracked.lock().unwrap();
	
		// Check and add directories that are not yet being watched.
		for (dir, &ok) in tracked.iter() {
		    if ok {
			continue;
		    }
	
		    match watcher.watch(dir, RecursiveMode::Recursive) {
			Ok(_) => {
			    tracked.insert(dir.clone(), true);
			    dir_errors.remove(dir);
			    update = true;
			}
			Err(e) => {
			    tracked.insert(dir.clone(), false);
			    let error = anyhow!("failed to monitor for changes: {}", e);
			    let error_ref: &(dyn std::error::Error + Send + Sync + 'static) = error.as_ref();
			    let boxed_error = Box::new(error_ref);
			    dir_errors.insert(dir.clone(), boxed_error);
			}
		    }
		}
	
		// Mark removed directories as not tracked and update errors.
		for dir in removed.iter() {
		    tracked.insert(dir.clone(), false);
		    let error = anyhow!("directory removed".to_string());
		    let error_ref: &(dyn std::error::Error + Send + Sync + 'static) = error.as_ref();
		    let boxed_error = Box::new(error_ref);
		    dir_errors.insert(dir.clone(), boxed_error);
		    update = true;
		}
	
		update
	    }
}