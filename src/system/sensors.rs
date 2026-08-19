use std::fs;
use std::path::Path;
use sysinfo::Components;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct SensorItem {
    pub label: String,
    pub temperature: f32,
    pub max: Option<f32>,
    pub critical: Option<f32>,
}

pub struct SensorCollector {
    pub components: Components,
    pub sensors: Vec<SensorItem>,
}

impl Default for SensorCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl SensorCollector {
    pub fn new() -> Self {
        let components = Components::new_with_refreshed_list();
        let mut collector = Self {
            components,
            sensors: Vec::new(),
        };
        collector.refresh();
        collector
    }

    pub fn refresh(&mut self) {
        self.components.refresh(true);
        self.sensors.clear();

        for comp in self.components.iter() {
            let label = comp.label().to_string();
            let temp = comp.temperature().unwrap_or(0.0);
            let max = comp.max();
            let crit = comp.critical();

            self.sensors.push(SensorItem {
                label,
                temperature: temp,
                max,
                critical: crit,
            });
        }

        // Fallback: Check /sys/class/thermal if sysinfo components yielded nothing
        if self.sensors.is_empty() {
            let thermal_path = Path::new("/sys/class/thermal");
            if thermal_path.exists() {
                if let Ok(entries) = fs::read_dir(thermal_path) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        let type_file = path.join("type");
                        let temp_file = path.join("temp");

                        if type_file.exists() && temp_file.exists() {
                            let label = fs::read_to_string(type_file)
                                .unwrap_or_else(|_| "Thermal Zone".to_string())
                                .trim()
                                .to_string();
                            if let Ok(raw_temp_str) = fs::read_to_string(temp_file) {
                                if let Ok(millicelsius) = raw_temp_str.trim().parse::<f32>() {
                                    let temp = millicelsius / 1000.0;
                                    self.sensors.push(SensorItem {
                                        label,
                                        temperature: temp,
                                        max: None,
                                        critical: Some(100.0),
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        self.sensors
            .sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));
    }
}
