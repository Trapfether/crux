use std::convert::TryFrom;
use std::fmt;
use std::sync::Arc;

use crate::http::{Method, Url};
use crate::middleware::{Middleware, Next};
use crate::{Config, Request, RequestBuilder, Response, Result};

/// An HTTP client, capable of sending `Request`s and running a middleware stack.
///
/// Can be optionally set with a base url.
///
/// # Examples
///
/// ```no_run
/// # #[async_std::main]
/// # async fn main() -> crux_http::Result<()> {
/// let client = crux_http::Client::new();
/// let res1 = client.recv_string(crux_http::get("https://httpbin.org/get"));
/// let res2 = client.recv_string(crux_http::get("https://httpbin.org/get"));
/// let (str1, str2) = futures_util::future::try_join(res1, res2).await?;
/// # Ok(()) }
/// ```
pub struct Client {
    config: Config,
    /// Holds the middleware stack.
    ///
    /// Note(Fishrock123): We do actually want this structure.
    /// The outer Arc allows us to clone in .send() without cloning the array.
    /// The Vec allows us to add middleware at runtime.
    /// The inner Arc-s allow us to implement Clone without sharing the vector with the parent.
    /// We don't use a Mutex around the Vec here because adding a middleware during execution should be an error.
    #[allow(clippy::rc_buffer)]
    middleware: Arc<Vec<Arc<dyn Middleware>>>,
}

impl Clone for Client {
    /// Clones the Client.
    ///
    /// This copies the middleware stack from the original, but shares
    /// the `HttpClient` and http client config of the original.
    /// Note that individual middleware in the middleware stack are
    /// still shared by reference.
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            middleware: Arc::new(self.middleware.iter().cloned().collect()),
        }
    }
}

impl fmt::Debug for Client {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Client {{}}")
    }
}

#[cfg(feature = "default-client")]
impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    /// Create a new `Client` instance.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// let client = crux_http::Client::new();
    ///
    /// let req = crux_http::get("https://httpbin.org/get");
    /// let res = client.send(req).await?;
    /// # Ok(()) }
    /// ```
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            middleware: Arc::new(vec![]),
        }
    }

    /// Push middleware onto the middleware stack.
    ///
    /// See the [middleware] submodule for more information on middleware.
    ///
    /// [middleware]: ../middleware/index.html
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// let req = crux_http::get("https://httpbin.org/get");
    /// let client = crux_http::client()
    ///     .with(crux_http::middleware::Redirect::default());
    /// let res = client.send(req).await?;
    /// # Ok(()) }
    /// ```
    pub fn with(mut self, middleware: impl Middleware) -> Self {
        let m = Arc::get_mut(&mut self.middleware)
            .expect("Registering middleware is not possible after the Client has been used");
        m.push(Arc::new(middleware));
        self
    }

    /// Send a `Request` using this client.
    ///
    /// Client middleware is run before per-request middleware.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// let req = crux_http::get("https://httpbin.org/get");
    /// let client = crux_http::client();
    /// let res = client.send(req).await?;
    /// # Ok(()) }
    /// ```
    pub async fn send(&self, req: impl Into<Request>) -> Result<Response> {
        todo!()
        /*
               let mut req: Request = req.into();
               let http_client = self.http_client.clone();
               let middleware = self.middleware.clone();

               let mw_stack = match req.take_middleware() {
                   Some(req_mw) => {
                       let mut mw = Vec::with_capacity(middleware.len() + req_mw.len());
                       mw.extend(middleware.iter().cloned());
                       mw.extend(req_mw);
                       Arc::new(mw)
                   }
                   None => middleware,
               };

               let next = Next::new(&mw_stack, &|req, client| {
                   Box::pin(async move {
                       let req: http_types::Request = req.into();
                       client.http_client.send(req).await.map(Into::into)
                   })
               });

               let client = Self {
                   config: self.config.clone(),
                   // Erase the middleware stack for the Client accessible from within middleware.
                   // This avoids gratuitous circular borrow & logic issues.
                   middleware: Arc::new(vec![]),
               };

               let res = next.run(req, client).await?;
               Ok(Response::new(res.into()))
        */
    }

    /// Submit a `Request` and get the response body as bytes.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// let req = crux_http::get("https://httpbin.org/get");
    /// let bytes = crux_http::client().recv_bytes(req).await?;
    /// assert!(bytes.len() > 0);
    /// # Ok(()) }
    /// ```
    pub async fn recv_bytes(&self, req: impl Into<Request>) -> Result<Vec<u8>> {
        let mut res = self.send(req.into()).await?;
        res.body_bytes().await
    }

    /// Submit a `Request` and get the response body as a string.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// let req = crux_http::get("https://httpbin.org/get");
    /// let string = crux_http::client().recv_string(req).await?;
    /// assert!(string.len() > 0);
    /// # Ok(()) }
    /// ```
    pub async fn recv_string(&self, req: impl Into<Request>) -> Result<String> {
        let mut res = self.send(req.into()).await?;
        res.body_string().await
    }

    /// Submit a `Request` and decode the response body from json into a struct.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use serde::{Deserialize, Serialize};
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// #[derive(Deserialize, Serialize)]
    /// struct Ip {
    ///     ip: String
    /// }
    ///
    /// let req = crux_http::get("https://api.ipify.org?format=json");
    /// let Ip { ip } = crux_http::client().recv_json(req).await?;
    /// assert!(ip.len() > 10);
    /// # Ok(()) }
    /// ```
    pub async fn recv_json<T: serde::de::DeserializeOwned>(
        &self,
        req: impl Into<Request>,
    ) -> Result<T> {
        let mut res = self.send(req.into()).await?;
        res.body_json::<T>().await
    }

    /// Submit a `Request` and decode the response body from form encoding into a struct.
    ///
    /// # Errors
    ///
    /// Any I/O error encountered while reading the body is immediately returned
    /// as an `Err`.
    ///
    /// If the body cannot be interpreted as valid json for the target type `T`,
    /// an `Err` is returned.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # use serde::{Deserialize, Serialize};
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// #[derive(Deserialize, Serialize)]
    /// struct Body {
    ///     apples: u32
    /// }
    ///
    /// let req = crux_http::get("https://api.example.com/v1/response");
    /// let Body { apples } = crux_http::client().recv_form(req).await?;
    /// # Ok(()) }
    /// ```
    pub async fn recv_form<T: serde::de::DeserializeOwned>(
        &self,
        req: impl Into<Request>,
    ) -> Result<T> {
        let mut res = self.send(req.into()).await?;
        res.body_form::<T>().await
    }

    /// Perform an HTTP `GET` request using the `Client` connection.
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Errors
    ///
    /// Returns errors from the middleware, http backend, and network sockets.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// let client = crux_http::client();
    /// let string = client.get("https://httpbin.org/get").recv_string().await?;
    /// # Ok(()) }
    /// ```
    pub fn get(&self, uri: impl AsRef<str>) -> RequestBuilder {
        RequestBuilder::new(Method::Get, self.url(uri)).with_client(self.clone())
    }

    /// Perform an HTTP `HEAD` request using the `Client` connection.
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Errors
    ///
    /// Returns errors from the middleware, http backend, and network sockets.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// let client = crux_http::client();
    /// let string = client.head("https://httpbin.org/head").recv_string().await?;
    /// # Ok(()) }
    /// ```
    pub fn head(&self, uri: impl AsRef<str>) -> RequestBuilder {
        RequestBuilder::new(Method::Head, self.url(uri)).with_client(self.clone())
    }

    /// Perform an HTTP `POST` request using the `Client` connection.
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Errors
    ///
    /// Returns errors from the middleware, http backend, and network sockets.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// let client = crux_http::client();
    /// let string = client.post("https://httpbin.org/post").recv_string().await?;
    /// # Ok(()) }
    /// ```
    pub fn post(&self, uri: impl AsRef<str>) -> RequestBuilder {
        RequestBuilder::new(Method::Post, self.url(uri)).with_client(self.clone())
    }

    /// Perform an HTTP `PUT` request using the `Client` connection.
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Errors
    ///
    /// Returns errors from the middleware, http backend, and network sockets.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// let client = crux_http::client();
    /// let string = client.put("https://httpbin.org/put").recv_string().await?;
    /// # Ok(()) }
    /// ```
    pub fn put(&self, uri: impl AsRef<str>) -> RequestBuilder {
        RequestBuilder::new(Method::Put, self.url(uri)).with_client(self.clone())
    }

    /// Perform an HTTP `DELETE` request using the `Client` connection.
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Errors
    ///
    /// Returns errors from the middleware, http backend, and network sockets.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// let client = crux_http::client();
    /// let string = client.delete("https://httpbin.org/delete").recv_string().await?;
    /// # Ok(()) }
    /// ```
    pub fn delete(&self, uri: impl AsRef<str>) -> RequestBuilder {
        RequestBuilder::new(Method::Delete, self.url(uri)).with_client(self.clone())
    }

    /// Perform an HTTP `CONNECT` request using the `Client` connection.
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Errors
    ///
    /// Returns errors from the middleware, http backend, and network sockets.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// let client = crux_http::client();
    /// let string = client.connect("https://httpbin.org/connect").recv_string().await?;
    /// # Ok(()) }
    /// ```
    pub fn connect(&self, uri: impl AsRef<str>) -> RequestBuilder {
        RequestBuilder::new(Method::Connect, self.url(uri)).with_client(self.clone())
    }

    /// Perform an HTTP `OPTIONS` request using the `Client` connection.
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Errors
    ///
    /// Returns errors from the middleware, http backend, and network sockets.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// let client = crux_http::client();
    /// let string = client.options("https://httpbin.org/options").recv_string().await?;
    /// # Ok(()) }
    /// ```
    pub fn options(&self, uri: impl AsRef<str>) -> RequestBuilder {
        RequestBuilder::new(Method::Options, self.url(uri)).with_client(self.clone())
    }

    /// Perform an HTTP `TRACE` request using the `Client` connection.
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Errors
    ///
    /// Returns errors from the middleware, http backend, and network sockets.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// let client = crux_http::client();
    /// let string = client.trace("https://httpbin.org/trace").recv_string().await?;
    /// # Ok(()) }
    /// ```
    pub fn trace(&self, uri: impl AsRef<str>) -> RequestBuilder {
        RequestBuilder::new(Method::Trace, self.url(uri)).with_client(self.clone())
    }

    /// Perform an HTTP `PATCH` request using the `Client` connection.
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Errors
    ///
    /// Returns errors from the middleware, http backend, and network sockets.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// let client = crux_http::client();
    /// let string = client.patch("https://httpbin.org/patch").recv_string().await?;
    /// # Ok(()) }
    /// ```
    pub fn patch(&self, uri: impl AsRef<str>) -> RequestBuilder {
        RequestBuilder::new(Method::Patch, self.url(uri)).with_client(self.clone())
    }

    /// Perform a HTTP request with the given verb using the `Client` connection.
    ///
    /// # Panics
    ///
    /// This will panic if a malformed URL is passed.
    ///
    /// # Errors
    ///
    /// Returns errors from the middleware, http backend, and network sockets.
    ///
    /// # Examples
    /// ```no_run
    /// # #[async_std::main]
    /// # async fn main() -> crux_http::Result<()> {
    /// use http_types::Method;
    /// let client = crux_http::client();
    /// let req = client.request(Method::Get, "http://httpbin.org/get");
    /// let res = client.send(req).await?;
    /// # Ok(()) }
    /// ```
    pub fn request(&self, verb: Method, uri: impl AsRef<str>) -> RequestBuilder {
        RequestBuilder::new(verb, self.url(uri)).with_client(self.clone())
    }

    /// Sets the base URL for this client. All request URLs will be relative to this URL.
    ///
    /// Note: a trailing slash is significant.
    /// Without it, the last path component is considered to be a “file” name
    /// to be removed to get at the “directory” that is used as the base.
    ///
    /// # Examples
    /// ```no_run
    /// # use http_types::Url;
    /// # fn main() -> http_types::Result<()> { async_std::task::block_on(async {
    /// let mut client = crux_http::client();
    /// client.set_base_url(Url::parse("http://example.com/api/v1/")?);
    /// client.get("posts.json").recv_json().await?; /// http://example.com/api/v1/posts.json
    /// # Ok(()) }) }
    /// ```
    #[deprecated(since = "6.5.0", note = "Please use `Config` instead")]
    pub fn set_base_url(&mut self, base: Url) {
        self.config.base_url = Some(base);
    }

    /// Get the current configuration.
    pub fn config(&self) -> &Config {
        &self.config
    }

    // private function to generate a url based on the base_path
    fn url(&self, uri: impl AsRef<str>) -> Url {
        match &self.config.base_url {
            None => uri.as_ref().parse().unwrap(),
            Some(base) => base.join(uri.as_ref()).unwrap(),
        }
    }
}

impl From<Config> for Client {
    fn from(mut config: Config) -> Self {
        Client {
            config,
            middleware: Arc::new(vec![]),
        }
    }
}

#[cfg(test)]
mod client_tests {
    use std::convert::TryInto;

    use super::Client;
    use super::Config;
    use crate::Url;

    #[test]
    fn base_url() {
        let base_url = Url::parse("http://example.com/api/v1/").unwrap();

        let client: Client = Config::new().set_base_url(base_url).try_into().unwrap();
        let url = client.url("posts.json");
        assert_eq!(url.as_str(), "http://example.com/api/v1/posts.json");
    }
}
