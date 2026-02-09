use serde::Serialize;

use super::runtime::RuntimeError;

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

    pub fn push<T: TryIntoFunctionParameters>(&mut self, value: &T) -> Result<(), RuntimeError> {
        self.0.extend(value.try_into_function_parameter()?.0);
        Ok(())
    }

    pub fn into_inner(self) -> Vec<serde_json::Value> {
        self.0
    }
}

pub trait TryIntoFunctionParameters {
    fn try_into_function_parameter(&self) -> Result<FunctionParameters, RuntimeError>;
}

impl<T> TryIntoFunctionParameters for T
where
    T: Serialize + ?Sized,
{
    fn try_into_function_parameter(&self) -> Result<FunctionParameters, RuntimeError> {
        let mut v: serde_json::Value = serde_json::to_value(self)?;

        if v.is_array() {
            // Avoid cloning the array
            let v = v
                .as_array_mut()
                .expect("Value should be array after is_array check")
                .drain(..)
                .collect();
            Ok(FunctionParameters(v))
        } else {
            Ok(FunctionParameters(vec![v]))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn test_tuple_parameters() {
        // Tuples serialize as arrays in serde_json
        test_into_function_params((5,));
        test_into_function_params((1, 2, 3, 4, 5, 6, 7, 8));

        // Test mixed types in tuples
        test_into_function_params((5, "hello"));
        test_into_function_params((true, 42, "world"));
        test_into_function_params((1.5, false, "test", 100));
    }

    #[test]
    fn test_tuple_conversion() {
        let result = (5, 3).try_into_function_parameter().unwrap();
        assert_eq!(result.0.len(), 2);
        assert_eq!(result.0[0], serde_json::json!(5));
        assert_eq!(result.0[1], serde_json::json!(3));

        let result = (10, 20, 30).try_into_function_parameter().unwrap();
        assert_eq!(result.0.len(), 3);
        assert_eq!(result.0[0], serde_json::json!(10));
        assert_eq!(result.0[1], serde_json::json!(20));
        assert_eq!(result.0[2], serde_json::json!(30));

        // Test vec types
        let result = ("hello", vec![1, 5, 8])
            .try_into_function_parameter()
            .unwrap();
        assert_eq!(result.0.len(), 2);
        assert_eq!(result.0[0], serde_json::json!("hello"));
        assert_eq!(result.0[1], serde_json::json!(vec![1, 5, 8]));

        // Test mixed types
        let result = (42, "hello", true).try_into_function_parameter().unwrap();
        assert_eq!(result.0.len(), 3);
        assert_eq!(result.0[0], serde_json::json!(42));
        assert_eq!(result.0[1], serde_json::json!("hello"));
        assert_eq!(result.0[2], serde_json::json!(true));

        // Test large tuple
        let result = (1, 2, 3, 4, 5, 6, 7, 8, 9, 10)
            .try_into_function_parameter()
            .unwrap();
        assert_eq!(result.0.len(), 10);
        for (i, val) in result.0.iter().enumerate() {
            assert_eq!(*val, serde_json::json!(i + 1));
        }
    }
}
