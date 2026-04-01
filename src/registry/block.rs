use crate::bindings::protobuf::core::sync::SyncBlockStates;
use crate::plugin::PluginImpl;
use jni::errors::{Error, ThrowRuntimeExAndDefault};
use jni::objects::{JByteArray, JClass};
use jni::EnvUnowned;
use prost::Message;
use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;
use rustc_hash::FxHashMap;

#[derive(Debug, Clone)]
pub enum PropertyType {
    Enum(Vec<String>),  // allowed enum values
    Integer(i32, i32),  // min, max
    Boolean,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PropertyValue {
    Enum(String),  // e.g., facing: "north"
    Integer(i32),  // e.g., age: 3
    Boolean(bool), // e.g., powered: true
}

#[derive(Debug, Clone)]
struct BlockState {
    pub id: i32,
    pub name: String,
    pub properties: BTreeMap<String, PropertyValue>,
}

#[derive(Debug, Clone)]
pub struct BlockTypeSchema {
    pub name: String,
    pub property_types: FxHashMap<String, PropertyType>,
}

// NEW: Groups all data for a single block type together for O(1) lookups
#[derive(Debug, Clone)]
pub struct BlockFamily {
    pub schema: BlockTypeSchema,
    pub default_properties: BTreeMap<String, PropertyValue>,
    // Fast O(1) lookup mapping a resolved property set directly to its State ID
    pub state_lookup: FxHashMap<BTreeMap<String, PropertyValue>, i32>
}

#[derive(Debug, Default)]
pub struct BlockStateRegistry {
    id_to_state: HashMap<i32, BlockState>,
    name_to_family: HashMap<String, BlockFamily>,
}

impl crate::example::plugin::block_registry::Host for PluginImpl {
    fn get_block_state(&mut self, name: String, props: Vec<(String, String)>) -> i32 {
        let block_state_registry = BLOCK_STATE_REGISTRY.get().expect("BlockStateRegistry is not initialized");
        block_state_registry.get_state_id(name.as_str(), props).unwrap_or(-1)
    }
}

impl BlockStateRegistry {
    fn get_state_id(&self, name: &str, overrides: Vec<(String, String)>) -> Option<i32> {
        let family = self.name_to_family.get(name)?;

        // Start with default properties (only 1 clone happens here)
        let mut props = family.default_properties.clone();

        // Parse and apply overrides in a single pass
        for (key, value_str) in overrides {
            if let Some(prop_type) = family.schema.property_types.get(&key) {
                let parsed_val = match prop_type {
                    PropertyType::Boolean => PropertyValue::Boolean(value_str == "true"),
                    PropertyType::Integer(min, max) => {
                        if let Ok(v) = value_str.parse::<i32>() {
                            if v >= *min && v <= *max {
                                PropertyValue::Integer(v)
                            } else {
                                continue;
                            }
                        } else {
                            continue;
                        }
                    }
                    PropertyType::Enum(valid) => {
                        if valid.contains(&value_str) {
                            PropertyValue::Enum(value_str)
                        } else {
                            continue;
                        }
                    }
                };
                props.insert(key, parsed_val);
            }
        }

        // O(1) direct hash lookup instead of O(N) linear scan!
        family.state_lookup.get(&props).copied()
    }
}

static BLOCK_STATE_REGISTRY: OnceLock<BlockStateRegistry> = OnceLock::new();

#[allow(non_snake_case)]
#[unsafe(no_mangle)]
pub extern "system" fn Java_de_cjdev_wasm_core_Universe_sync_1block_1states<'caller>(
    mut unowned_env: EnvUnowned<'caller>,
    _class: JClass<'caller>,
    message: JByteArray<'caller>,
) {
    unowned_env.with_env(|env| {
        let buf: Vec<u8> = env.convert_byte_array(message)?;
        let registry = build_registry_from_proto(&buf);
        BLOCK_STATE_REGISTRY.set(registry).expect("BlockStateRegistry already initialized");
        Ok::<(), Error>(())
    }).resolve::<ThrowRuntimeExAndDefault>();
}

fn build_registry_from_proto(buf: &[u8]) -> BlockStateRegistry {
    let sync_states = SyncBlockStates::decode(&*buf).expect("Failed to decode protobuf");
    let mut registry = BlockStateRegistry::default();

    // Temporary structures for schema
    let mut name_to_properties: HashMap<String, HashMap<String, Vec<String>>> = HashMap::new();
    let mut schemas: HashMap<String, BlockTypeSchema> = HashMap::new();

    // First pass: gather properties
    for state in &sync_states.states {
        let props_map = name_to_properties.entry(state.name.clone()).or_default();
        for property in &state.properties {
            props_map.entry(property.name.clone())
                .or_insert_with(Vec::new)
                .push(property.value.clone());
        }
    }

    // Build schemas
    for (name, prop_values) in name_to_properties.iter() {
        let mut schema_props = FxHashMap::default();
        for (key, values) in prop_values {
            let mut unique_values = values.clone();
            unique_values.sort();
            unique_values.dedup();

            let prop_type = if unique_values.len() == 2 && unique_values.contains(&"true".to_string()) {
                PropertyType::Boolean
            } else if unique_values.iter().all(|v| v.parse::<i32>().is_ok()) {
                let min = unique_values.iter().filter_map(|v| v.parse::<i32>().ok()).min().unwrap_or(0);
                let max = unique_values.iter().filter_map(|v| v.parse::<i32>().ok()).max().unwrap_or(0);
                PropertyType::Integer(min, max)
            } else {
                PropertyType::Enum(unique_values)
            };

            schema_props.insert(key.clone(), prop_type);
        }
        schemas.insert(name.clone(), BlockTypeSchema { name: name.clone(), property_types: schema_props });
    }

    // Second pass: register states and build families
    for state in &sync_states.states {
        let schema = schemas.get(&state.name).unwrap();

        let mut properties = BTreeMap::new();
        for property in &state.properties {
            let prop_type = schema.property_types.get(&property.name).unwrap();
            let v = &property.value;
            let value = match prop_type {
                PropertyType::Boolean => PropertyValue::Boolean(v == "true"),
                PropertyType::Integer(_, _) => PropertyValue::Integer(v.parse::<i32>().unwrap_or(0)),
                PropertyType::Enum(_) => PropertyValue::Enum(v.clone()),
            };
            properties.insert(property.name.clone(), value);
        }

        let block_state = BlockState {
            id: state.id,
            name: state.name.clone(),
            properties: properties.clone(),
        };

        registry.id_to_state.insert(block_state.id, block_state.clone());

        let family = registry.name_to_family.entry(state.name.clone()).or_insert_with(|| BlockFamily {
            schema: schema.clone(),
            default_properties: BTreeMap::new(),
            state_lookup: FxHashMap::default(),
        });

        // Register the specific property combination to this state ID
        family.state_lookup.insert(properties.clone(), state.id);

        if state.is_default {
            family.default_properties = properties;
        }
    }

    registry
}
