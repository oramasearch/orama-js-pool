use serde::Serialize;

use crate::JSRunnerError;

pub struct FunctionParameters(pub(crate) Vec<serde_json::Value>);

impl Default for FunctionParameters {
    fn default() -> Self {
        Self::new()
    }
}

impl FunctionParameters {
    pub fn new() -> Self {
        Self(vec![])
    }

    pub fn push<T: TryIntoFunctionParameters>(&mut self, value: T) -> Result<(), JSRunnerError> {
        self.0.extend(value.try_into_function_parameter()?.0);
        Ok(())
    }

    pub fn into_inner(self) -> Vec<serde_json::Value> {
        self.0
    }
}

pub trait TryIntoFunctionParameters {
    fn try_into_function_parameter(self) -> Result<FunctionParameters, JSRunnerError>;
}

impl<T> TryIntoFunctionParameters for T
where
    T: Serialize,
{
    fn try_into_function_parameter(self) -> Result<FunctionParameters, JSRunnerError> {
        let mut v: serde_json::Value = serde_json::to_value(self)?;

        if v.is_array() {
            // Avoid cloning the array
            let v = v.as_array_mut().unwrap().drain(..).collect();
            Ok(FunctionParameters(v))
        } else {
            Ok(FunctionParameters(vec![v]))
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::TryIntoFunctionParameters;

    fn test_into_function_params<Params: TryIntoFunctionParameters>(_: Params) {}

    #[test]
    fn test_into_function_parameters() {
        test_into_function_params(5);
        test_into_function_params("5");
        test_into_function_params("5".to_string());
        test_into_function_params(true);
        test_into_function_params(Some(true));
        test_into_function_params(vec![true]);
        test_into_function_params(vec!["true".to_string()]);
        test_into_function_params(vec!["true".to_string(), "false".to_string()]);

        let params: Vec<serde_json::Value> = vec!["true".into(), 1.into()];
        test_into_function_params(params);
    }
}
