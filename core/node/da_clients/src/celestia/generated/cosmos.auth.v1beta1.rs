// This file is @generated by prost-build.
/// BaseAccount defines a base account type. It contains all the necessary fields
/// for basic account functionality. Any custom account type should extend this
/// type for additional functionality (e.g. vesting).
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct BaseAccount {
    #[prost(string, tag = "1")]
    pub address: ::prost::alloc::string::String,
    #[prost(message, optional, tag = "2")]
    pub pub_key: ::core::option::Option<::pbjson_types::Any>,
    #[prost(uint64, tag = "3")]
    pub account_number: u64,
    #[prost(uint64, tag = "4")]
    pub sequence: u64,
}
impl ::prost::Name for BaseAccount {
    const NAME: &'static str = "BaseAccount";
    const PACKAGE: &'static str = "cosmos.auth.v1beta1";
    fn full_name() -> ::prost::alloc::string::String {
        "cosmos.auth.v1beta1.BaseAccount".into()
    }
    fn type_url() -> ::prost::alloc::string::String {
        "/cosmos.auth.v1beta1.BaseAccount".into()
    }
}
/// Params defines the parameters for the auth module.
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct Params {
    #[prost(uint64, tag = "1")]
    pub max_memo_characters: u64,
    #[prost(uint64, tag = "2")]
    pub tx_sig_limit: u64,
    #[prost(uint64, tag = "3")]
    pub tx_size_cost_per_byte: u64,
    #[prost(uint64, tag = "4")]
    pub sig_verify_cost_ed25519: u64,
    #[prost(uint64, tag = "5")]
    pub sig_verify_cost_secp256k1: u64,
}
impl ::prost::Name for Params {
    const NAME: &'static str = "Params";
    const PACKAGE: &'static str = "cosmos.auth.v1beta1";
    fn full_name() -> ::prost::alloc::string::String {
        "cosmos.auth.v1beta1.Params".into()
    }
    fn type_url() -> ::prost::alloc::string::String {
        "/cosmos.auth.v1beta1.Params".into()
    }
}
/// QueryAccountRequest is the request type for the Query/Account RPC method.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct QueryAccountRequest {
    /// address defines the address to query for.
    #[prost(string, tag = "1")]
    pub address: ::prost::alloc::string::String,
}
impl ::prost::Name for QueryAccountRequest {
    const NAME: &'static str = "QueryAccountRequest";
    const PACKAGE: &'static str = "cosmos.auth.v1beta1";
    fn full_name() -> ::prost::alloc::string::String {
        "cosmos.auth.v1beta1.QueryAccountRequest".into()
    }
    fn type_url() -> ::prost::alloc::string::String {
        "/cosmos.auth.v1beta1.QueryAccountRequest".into()
    }
}
/// QueryAccountResponse is the response type for the Query/Account RPC method.
#[derive(Clone, PartialEq, ::prost::Message)]
pub struct QueryAccountResponse {
    /// account defines the account of the corresponding address.
    #[prost(message, optional, tag = "1")]
    pub account: ::core::option::Option<::pbjson_types::Any>,
}
impl ::prost::Name for QueryAccountResponse {
    const NAME: &'static str = "QueryAccountResponse";
    const PACKAGE: &'static str = "cosmos.auth.v1beta1";
    fn full_name() -> ::prost::alloc::string::String {
        "cosmos.auth.v1beta1.QueryAccountResponse".into()
    }
    fn type_url() -> ::prost::alloc::string::String {
        "/cosmos.auth.v1beta1.QueryAccountResponse".into()
    }
}
/// QueryParamsRequest is the request type for the Query/Params RPC method.
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct QueryParamsRequest {}
impl ::prost::Name for QueryParamsRequest {
    const NAME: &'static str = "QueryParamsRequest";
    const PACKAGE: &'static str = "cosmos.auth.v1beta1";
    fn full_name() -> ::prost::alloc::string::String {
        "cosmos.auth.v1beta1.QueryParamsRequest".into()
    }
    fn type_url() -> ::prost::alloc::string::String {
        "/cosmos.auth.v1beta1.QueryParamsRequest".into()
    }
}
/// QueryParamsResponse is the response type for the Query/Params RPC method.
#[derive(Clone, Copy, PartialEq, ::prost::Message)]
pub struct QueryParamsResponse {
    /// params defines the parameters of the module.
    #[prost(message, optional, tag = "1")]
    pub params: ::core::option::Option<Params>,
}
impl ::prost::Name for QueryParamsResponse {
    const NAME: &'static str = "QueryParamsResponse";
    const PACKAGE: &'static str = "cosmos.auth.v1beta1";
    fn full_name() -> ::prost::alloc::string::String {
        "cosmos.auth.v1beta1.QueryParamsResponse".into()
    }
    fn type_url() -> ::prost::alloc::string::String {
        "/cosmos.auth.v1beta1.QueryParamsResponse".into()
    }
}
/// Generated client implementations.
pub mod query_client {
    #![allow(
        unused_variables,
        dead_code,
        missing_docs,
        clippy::wildcard_imports,
        clippy::let_unit_value,
    )]
    use tonic::codegen::*;
    use tonic::codegen::http::Uri;
    /// Query defines the gRPC querier service.
    #[derive(Debug, Clone)]
    pub struct QueryClient<T> {
        inner: tonic::client::Grpc<T>,
    }
    impl QueryClient<tonic::transport::Channel> {
        /// Attempt to create a new client by connecting to a given endpoint.
        pub async fn connect<D>(dst: D) -> Result<Self, tonic::transport::Error>
        where
            D: TryInto<tonic::transport::Endpoint>,
            D::Error: Into<StdError>,
        {
            let conn = tonic::transport::Endpoint::new(dst)?.connect().await?;
            Ok(Self::new(conn))
        }
    }
    impl<T> QueryClient<T>
    where
        T: tonic::client::GrpcService<tonic::body::BoxBody>,
        T::Error: Into<StdError>,
        T::ResponseBody: Body<Data = Bytes> + std::marker::Send + 'static,
        <T::ResponseBody as Body>::Error: Into<StdError> + std::marker::Send,
    {
        pub fn new(inner: T) -> Self {
            let inner = tonic::client::Grpc::new(inner);
            Self { inner }
        }
        pub fn with_origin(inner: T, origin: Uri) -> Self {
            let inner = tonic::client::Grpc::with_origin(inner, origin);
            Self { inner }
        }
        pub fn with_interceptor<F>(
            inner: T,
            interceptor: F,
        ) -> QueryClient<InterceptedService<T, F>>
        where
            F: tonic::service::Interceptor,
            T::ResponseBody: Default,
            T: tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
                Response = http::Response<
                    <T as tonic::client::GrpcService<tonic::body::BoxBody>>::ResponseBody,
                >,
            >,
            <T as tonic::codegen::Service<
                http::Request<tonic::body::BoxBody>,
            >>::Error: Into<StdError> + std::marker::Send + std::marker::Sync,
        {
            QueryClient::new(InterceptedService::new(inner, interceptor))
        }
        /// Compress requests with the given encoding.
        ///
        /// This requires the server to support it otherwise it might respond with an
        /// error.
        #[must_use]
        pub fn send_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.send_compressed(encoding);
            self
        }
        /// Enable decompressing responses.
        #[must_use]
        pub fn accept_compressed(mut self, encoding: CompressionEncoding) -> Self {
            self.inner = self.inner.accept_compressed(encoding);
            self
        }
        /// Limits the maximum size of a decoded message.
        ///
        /// Default: `4MB`
        #[must_use]
        pub fn max_decoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_decoding_message_size(limit);
            self
        }
        /// Limits the maximum size of an encoded message.
        ///
        /// Default: `usize::MAX`
        #[must_use]
        pub fn max_encoding_message_size(mut self, limit: usize) -> Self {
            self.inner = self.inner.max_encoding_message_size(limit);
            self
        }
        /// Account returns account details based on address.
        pub async fn account(
            &mut self,
            request: impl tonic::IntoRequest<super::QueryAccountRequest>,
        ) -> std::result::Result<
            tonic::Response<super::QueryAccountResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/cosmos.auth.v1beta1.Query/Account",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("cosmos.auth.v1beta1.Query", "Account"));
            self.inner.unary(req, path, codec).await
        }
        /// Params queries all parameters.
        pub async fn params(
            &mut self,
            request: impl tonic::IntoRequest<super::QueryParamsRequest>,
        ) -> std::result::Result<
            tonic::Response<super::QueryParamsResponse>,
            tonic::Status,
        > {
            self.inner
                .ready()
                .await
                .map_err(|e| {
                    tonic::Status::unknown(
                        format!("Service was not ready: {}", e.into()),
                    )
                })?;
            let codec = tonic::codec::ProstCodec::default();
            let path = http::uri::PathAndQuery::from_static(
                "/cosmos.auth.v1beta1.Query/Params",
            );
            let mut req = request.into_request();
            req.extensions_mut()
                .insert(GrpcMethod::new("cosmos.auth.v1beta1.Query", "Params"));
            self.inner.unary(req, path, codec).await
        }
    }
}
