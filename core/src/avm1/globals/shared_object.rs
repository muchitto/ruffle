use crate::avm1::activation::Activation;
use crate::avm1::error::Error;
use crate::avm1::function::{Executable, FunctionObject};
use crate::avm1::{AvmString, Object, TObject, Value};
use crate::avm_warn;
use enumset::EnumSet;
use gc_arena::MutationContext;

use crate::avm1::object::shared_object::SharedObject;

use json::JsonValue;

pub fn delete_all<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    _this: Object<'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    avm_warn!(activation, "SharedObject.deleteAll() not implemented");
    Ok(Value::Undefined)
}

pub fn get_disk_usage<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    _this: Object<'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    avm_warn!(activation, "SharedObject.getDiskUsage() not implemented");
    Ok(Value::Undefined)
}

/// Serialize an Object and any children to a JSON object
/// It would be best if this was implemented via serde but due to avm and context it can't
/// Undefined fields aren't serialized
fn recursive_serialize<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    obj: Object<'gc>,
    json_obj: &mut JsonValue,
) {
    for k in &obj.get_keys(activation) {
        if let Ok(elem) = obj.get(k, activation) {
            match elem {
                Value::Undefined => {}
                Value::Null => json_obj[k] = JsonValue::Null,
                Value::Bool(b) => json_obj[k] = b.into(),
                Value::Number(f) => json_obj[k] = f.into(),
                Value::String(s) => json_obj[k] = s.to_string().into(),
                Value::Object(o) => {
                    // Don't attempt to serialize functions
                    let function = activation.context.avm1.prototypes.function;
                    if !o
                        .is_instance_of(activation, o, function)
                        .unwrap_or_default()
                    {
                        let mut sub_data_json = JsonValue::new_object();
                        recursive_serialize(activation, o, &mut sub_data_json);
                        json_obj[k] = sub_data_json;
                    }
                }
            }
        }
    }
}

/// Deserialize an Object and any children from a JSON object
/// It would be best if this was implemented via serde but due to avm and context it can't
/// Undefined fields aren't deserialized
fn recursive_deserialize<'gc>(
    json_obj: JsonValue,
    activation: &mut Activation<'_, 'gc, '_>,
    object: Object<'gc>,
) {
    for entry in json_obj.entries() {
        match entry.1 {
            JsonValue::Null => {
                object.define_value(
                    activation.context.gc_context,
                    entry.0,
                    Value::Null,
                    EnumSet::empty(),
                );
            }
            JsonValue::Short(s) => {
                let val: String = s.as_str().to_string();
                object.define_value(
                    activation.context.gc_context,
                    entry.0,
                    Value::String(AvmString::new(activation.context.gc_context, val)),
                    EnumSet::empty(),
                );
            }
            JsonValue::String(s) => {
                object.define_value(
                    activation.context.gc_context,
                    entry.0,
                    Value::String(AvmString::new(activation.context.gc_context, s.clone())),
                    EnumSet::empty(),
                );
            }
            JsonValue::Number(f) => {
                let val: f64 = f.clone().into();
                object.define_value(
                    activation.context.gc_context,
                    entry.0,
                    Value::Number(val),
                    EnumSet::empty(),
                );
            }
            JsonValue::Boolean(b) => {
                object.define_value(
                    activation.context.gc_context,
                    entry.0,
                    Value::Bool(*b),
                    EnumSet::empty(),
                );
            }
            JsonValue::Object(o) => {
                let prototype = activation.context.avm1.prototypes.object;
                if let Ok(obj) = prototype.create_bare_object(activation, prototype) {
                    recursive_deserialize(JsonValue::Object(o.clone()), activation, obj);

                    object.define_value(
                        activation.context.gc_context,
                        entry.0,
                        Value::Object(obj),
                        EnumSet::empty(),
                    );
                }
            }
            JsonValue::Array(_) => {}
        }
    }
}

pub fn get_local<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    _this: Object<'gc>,
    args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    let name = args
        .get(0)
        .unwrap_or(&Value::Undefined)
        .to_owned()
        .coerce_to_string(activation)?
        .to_string();

    //Check if this is referencing an existing shared object
    if let Some(so) = activation.context.shared_objects.get(&name) {
        return Ok(Value::Object(*so));
    }

    if args.len() > 1 {
        avm_warn!(
            activation,
            "SharedObject.getLocal() doesn't support localPath or secure yet"
        );
    }

    // Data property only should exist when created with getLocal/Remote
    let constructor = activation.context.avm1.prototypes.shared_object_constructor;
    let this = constructor.construct(activation, &[])?;

    // Set the internal name
    let obj_so = this.as_shared_object().unwrap();
    obj_so.set_name(activation.context.gc_context, name.to_string());

    // Create the data object
    let prototype = activation.context.avm1.prototypes.object;
    let data = prototype.create_bare_object(activation, prototype)?;

    // Load the data object from storage if it existed prior
    if let Some(saved) = activation.context.storage.get_string(&name) {
        if let Ok(json_data) = json::parse(&saved) {
            recursive_deserialize(json_data, activation, data);
        }
    }

    this.define_value(
        activation.context.gc_context,
        "data",
        data.into(),
        EnumSet::empty(),
    );

    activation.context.shared_objects.insert(name, this);

    Ok(this.into())
}

pub fn get_remote<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    _this: Object<'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    avm_warn!(activation, "SharedObject.getRemote() not implemented");
    Ok(Value::Undefined)
}

pub fn get_max_size<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    _this: Object<'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    avm_warn!(activation, "SharedObject.getMaxSize() not implemented");
    Ok(Value::Undefined)
}

pub fn add_listener<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    _this: Object<'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    avm_warn!(activation, "SharedObject.addListener() not implemented");
    Ok(Value::Undefined)
}

pub fn remove_listener<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    _this: Object<'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    avm_warn!(activation, "SharedObject.removeListener() not implemented");
    Ok(Value::Undefined)
}

pub fn create_shared_object_object<'gc>(
    gc_context: MutationContext<'gc, '_>,
    shared_object_proto: Object<'gc>,
    fn_proto: Option<Object<'gc>>,
) -> Object<'gc> {
    let shared_obj = FunctionObject::constructor(
        gc_context,
        Executable::Native(constructor),
        fn_proto,
        shared_object_proto,
    );
    let mut object = shared_obj.as_script_object().unwrap();

    object.force_set_function(
        "deleteAll",
        delete_all,
        gc_context,
        EnumSet::empty(),
        fn_proto,
    );

    object.force_set_function(
        "getDiskUsage",
        get_disk_usage,
        gc_context,
        EnumSet::empty(),
        fn_proto,
    );

    object.force_set_function(
        "getLocal",
        get_local,
        gc_context,
        EnumSet::empty(),
        fn_proto,
    );

    object.force_set_function(
        "getRemote",
        get_remote,
        gc_context,
        EnumSet::empty(),
        fn_proto,
    );

    object.force_set_function(
        "getMaxSize",
        get_max_size,
        gc_context,
        EnumSet::empty(),
        fn_proto,
    );

    object.force_set_function(
        "addListener",
        add_listener,
        gc_context,
        EnumSet::empty(),
        fn_proto,
    );

    object.force_set_function(
        "removeListener",
        remove_listener,
        gc_context,
        EnumSet::empty(),
        fn_proto,
    );

    shared_obj
}

pub fn clear<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    this: Object<'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    let data = this.get("data", activation)?.coerce_to_object(activation);

    for k in &data.get_keys(activation) {
        data.delete(activation, k);
    }

    let so = this.as_shared_object().unwrap();
    let name = so.get_name();

    activation.context.storage.remove_key(&name);

    Ok(Value::Undefined)
}

pub fn close<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    _this: Object<'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    avm_warn!(activation, "SharedObject.close() not implemented");
    Ok(Value::Undefined)
}

pub fn connect<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    _this: Object<'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    avm_warn!(activation, "SharedObject.connect() not implemented");
    Ok(Value::Undefined)
}

pub fn flush<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    this: Object<'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    let data = this.get("data", activation)?.coerce_to_object(activation);

    let mut data_json = JsonValue::new_object();
    recursive_serialize(activation, data, &mut data_json);

    let this_obj = this.as_shared_object().unwrap();
    let name = this_obj.get_name();

    Ok(activation
        .context
        .storage
        .put_string(&name, data_json.dump())
        .into())
}

pub fn get_size<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    _this: Object<'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    avm_warn!(activation, "SharedObject.getSize() not implemented");
    Ok(Value::Undefined)
}

pub fn send<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    _this: Object<'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    avm_warn!(activation, "SharedObject.send() not implemented");
    Ok(Value::Undefined)
}

pub fn set_fps<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    _this: Object<'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    avm_warn!(activation, "SharedObject.setFps() not implemented");
    Ok(Value::Undefined)
}

pub fn on_status<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    _this: Object<'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    avm_warn!(activation, "SharedObject.onStatus() not implemented");
    Ok(Value::Undefined)
}

pub fn on_sync<'gc>(
    activation: &mut Activation<'_, 'gc, '_>,
    _this: Object<'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    avm_warn!(activation, "SharedObject.onSync() not implemented");
    Ok(Value::Undefined)
}

pub fn create_proto<'gc>(
    gc_context: MutationContext<'gc, '_>,
    proto: Object<'gc>,
    fn_proto: Object<'gc>,
) -> Object<'gc> {
    let shared_obj = SharedObject::empty_shared_obj(gc_context, Some(proto));
    let mut object = shared_obj.as_script_object().unwrap();

    object.force_set_function("clear", clear, gc_context, EnumSet::empty(), Some(fn_proto));

    object.force_set_function("close", close, gc_context, EnumSet::empty(), Some(fn_proto));

    object.force_set_function(
        "connect",
        connect,
        gc_context,
        EnumSet::empty(),
        Some(fn_proto),
    );

    object.force_set_function("flush", flush, gc_context, EnumSet::empty(), Some(fn_proto));

    object.force_set_function(
        "getSize",
        get_size,
        gc_context,
        EnumSet::empty(),
        Some(fn_proto),
    );

    object.force_set_function("send", send, gc_context, EnumSet::empty(), Some(fn_proto));

    object.force_set_function(
        "setFps",
        set_fps,
        gc_context,
        EnumSet::empty(),
        Some(fn_proto),
    );

    object.force_set_function(
        "onStatus",
        on_status,
        gc_context,
        EnumSet::empty(),
        Some(fn_proto),
    );

    object.force_set_function(
        "onSync",
        on_sync,
        gc_context,
        EnumSet::empty(),
        Some(fn_proto),
    );

    shared_obj.into()
}

pub fn constructor<'gc>(
    _activation: &mut Activation<'_, 'gc, '_>,
    _this: Object<'gc>,
    _args: &[Value<'gc>],
) -> Result<Value<'gc>, Error<'gc>> {
    Ok(Value::Undefined)
}
