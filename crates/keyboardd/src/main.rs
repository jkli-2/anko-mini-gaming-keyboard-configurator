fn main() -> Result<(), Box<dyn std::error::Error>> {
    keyboardd::run_service()?;
    Ok(())
}
