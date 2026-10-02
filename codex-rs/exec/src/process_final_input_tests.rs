//! Regression coverage for input failures returning through executable cleanup.

use super::load_output_schema;
use super::resolve_prompt;

#[test]
fn missing_output_schema_returns_an_error_instead_of_exiting() -> anyhow::Result<()> {
    let directory = tempfile::tempdir()?;
    let result = load_output_schema(Some(directory.path().join("missing.json")));
    let Err(error) = result else {
        panic!("missing schema must return an error");
    };
    assert!(
        error
            .to_string()
            .contains("Failed to read output schema file")
    );
    Ok(())
}

#[test]
fn invalid_output_schema_returns_an_error_instead_of_exiting() -> anyhow::Result<()> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("schema.json");
    std::fs::write(&path, "{invalid}")?;
    let result = load_output_schema(Some(path));
    let Err(error) = result else {
        panic!("invalid schema must return an error");
    };
    assert!(error.to_string().contains("is not valid JSON"));
    Ok(())
}

#[test]
fn direct_prompt_and_absent_schema_still_succeed() -> anyhow::Result<()> {
    assert_eq!(resolve_prompt(Some("hello".to_string()))?, "hello");
    assert!(load_output_schema(None)?.is_none());
    Ok(())
}
