#[tokio::main]
async fn main() {
    yoyo_ds_harness::run_cli().await;
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_version_constant_accessible() {
        let version = yoyo_ds_harness::VERSION;
        assert!(!version.is_empty(), "VERSION must not be empty");
        assert!(
            version.starts_with("0."),
            "VERSION should be 0.x.y, got: {version}"
        );
    }
}
