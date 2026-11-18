use std::sync::Mutex;

use ort::environment::Environment;

use crate::EngineError;

static ENVIRONMENT: Mutex<Option<Environment>> = Mutex::new(None);

pub(crate) fn environment() -> Result<Environment, EngineError> {
    let mut shared = ENVIRONMENT
        .lock()
        .map_err(|_| EngineError::Model("the ONNX runtime lock is poisoned".to_owned()))?;
    if let Some(environment) = shared.as_ref() {
        return Ok(environment.clone());
    }
    let environment = ort::init()
        .build()
        .map_err(|e| EngineError::Model(e.to_string()))?;
    *shared = Some(environment.clone());
    Ok(environment)
}
