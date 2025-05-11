use tokio::sync::Mutex;

// TODO use pool of connections instead of one
pub(crate) struct RedisSingleConnection {
    conn: Mutex<redis::aio::MultiplexedConnection>,
}

impl RedisSingleConnection {
    pub(crate) async fn new(endpoint: String) -> Self {
        let client =
            redis::Client::open(format!("redis://{}/", endpoint)).expect("Cannot connect to redis");

        RedisSingleConnection {
            conn: Mutex::new(
                client
                    .get_multiplexed_async_connection()
                    .await
                    .expect("Cannot get async connection"),
            ),
        }
    }

    pub(crate) async fn store(&self, short: String, url: String) -> bool {
        // Set key=short with value=url if not set yet atomically
        // TODO think about for how much time to store?
        match self
            .conn
            .lock()
            .await
            .send_packed_command(
                redis::cmd("SET")
                    .arg(short)
                    .arg(url)
                    .arg("EX")
                    .arg(3600)
                    .arg("NX"),
            )
            .await
        {
            Ok(resp) => match resp {
                redis::Value::Okay => true,
                redis::Value::Nil => false,
                _ => {
                    log::warn!("Response from redis store is not OK, nor nil");
                    false
                }
            },
            Err(e) => {
                log::error!("Error to store in redis: {}", e);
                false
            }
        }
    }

    pub(crate) async fn fetch(&self, short: &str) -> Option<String> {
        match self
            .conn
            .lock()
            .await
            .send_packed_command(redis::cmd("GET").arg(short))
            .await
        {
            Ok(resp) => match resp {
                redis::Value::SimpleString(s) => Some(s),
                redis::Value::Nil => None,
                redis::Value::BulkString(s) => match String::from_utf8(s) {
                    Ok(s) => Some(s),
                    Err(_) => {
                        log::warn!("Non utf-8 url is stored");
                        None
                    }
                },
                _ => {
                    log::warn!("Stored value is not string");
                    None
                }
            },
            Err(e) => {
                log::error!("Error to fetch from redis: {}", e);
                None
            }
        }
    }
}
