use super::*;

pub fn dispatch_iac_app_request(
    service: &CloudIacAppService,
    request: HttpRequest,
) -> HttpResponse {
    dispatch_hyper_adapter_request(request, &service.router, &service.middleware)
}

pub fn serve_iac_app_on_listener(
    listener: StdTcpListener,
    service: CloudIacAppService,
) -> Result<(), CloudIacAppError> {
    let (router, middleware, server_config) = service.into_serve_parts();
    serve_on_std_listener(
        listener,
        Arc::new(router),
        Arc::new(middleware),
        server_config,
    )?;
    Ok(())
}

pub fn serve_bounded_iac_app_on_listener(
    listener: StdTcpListener,
    service: CloudIacAppService,
    max_connections: usize,
) -> Result<(), CloudIacAppError> {
    let (router, middleware, server_config) = service.into_serve_parts();
    serve_n_connections_on_std_listener(
        listener,
        Arc::new(router),
        Arc::new(middleware),
        server_config,
        max_connections,
    )?;
    Ok(())
}

pub fn run_iac_app(config: CloudIacAppConfig) -> Result<(), CloudIacAppError> {
    run_iac_app_with_termination(config, false)
}

pub(super) fn run_iac_app_with_termination(
    config: CloudIacAppConfig,
    process_signals: bool,
) -> Result<(), CloudIacAppError> {
    let authz_provider = config.module_registry_authz_provider()?;
    let service =
        build_iac_app_service_from_release_index_path(&config.release_index_path, authz_provider)?;
    let listener = StdTcpListener::bind(config.bind_addr)
        .map_err(|error| CloudIacAppError::Bind(error.to_string()))?;
    if process_signals {
        let (router, middleware, server_config) = service.into_serve_parts();
        let report = serve_with_signals_on_std_listener(
            listener,
            Arc::new(router),
            Arc::new(middleware),
            server_config,
            ServingControl::new(ServingLimits::default()),
        )?;
        finish_serving(report)
    } else {
        serve_iac_app_on_listener(listener, service)
    }
}

fn finish_serving(
    report: http_runtime_hyper_adapter::ServingReport,
) -> Result<(), CloudIacAppError> {
    report.into_result().map_err(Into::into)
}

pub fn run_iac_app_from_env() -> Result<(), CloudIacAppError> {
    let config = CloudIacAppConfig::from_env_pairs(std::env::vars())?;
    run_iac_app(config)
}

pub fn run_iac_app_with_signals_from_env() -> Result<(), CloudIacAppError> {
    let config = CloudIacAppConfig::from_env_pairs(std::env::vars())?;
    run_iac_app_with_termination(config, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use http_runtime_hyper_adapter::{ServingOutcome, ServingReport};

    #[test]
    fn executable_serving_result_refuses_incomplete_and_failed_drains() {
        for outcome in [
            ServingOutcome::Drained,
            ServingOutcome::DeadlineExceeded,
            ServingOutcome::InfrastructureFailure,
        ] {
            let control = ServingControl::new(ServingLimits::default());
            let report = ServingReport {
                outcome,
                snapshot: control.snapshot(),
                completion: control,
                failure: None,
            };
            assert_eq!(
                finish_serving(report).is_ok(),
                outcome == ServingOutcome::Drained
            );
        }
    }
}
