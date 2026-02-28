mod common;

#[cfg(test)]
mod error_tests {
    use matrix_bridge_wechat::error::{BridgeError, MatrixError, WeChatError, CryptoError};
    
    #[test]
    fn test_matrix_error_conversion() {
        let matrix_err = MatrixError::Api {
            code: "M_UNKNOWN".to_string(),
            message: "Unknown error".to_string(),
        };
        let bridge_err: BridgeError = matrix_err.into();
        
        match bridge_err {
            BridgeError::Matrix(e) => {
                assert!(matches!(e, MatrixError::Api { .. }));
            }
            _ => panic!("Expected Matrix error"),
        }
    }
    
    #[test]
    fn test_wechat_error_conversion() {
        let wechat_err = WeChatError::LoginRequired;
        let bridge_err: BridgeError = wechat_err.into();
        
        match bridge_err {
            BridgeError::WeChat(e) => {
                assert!(matches!(e, WeChatError::LoginRequired));
            }
            _ => panic!("Expected WeChat error"),
        }
    }
    
    #[test]
    fn test_crypto_error_conversion() {
        let crypto_err = CryptoError::KeyNotFound("test_key".to_string());
        let bridge_err: BridgeError = crypto_err.into();
        
        match bridge_err {
            BridgeError::Crypto(msg) => {
                assert!(msg.contains("test_key"));
            }
            _ => panic!("Expected Crypto error"),
        }
    }
    
    #[test]
    fn test_error_display() {
        let err = BridgeError::Timeout("operation timed out".to_string());
        let msg = format!("{}", err);
        assert!(msg.contains("Timeout"));
        assert!(msg.contains("operation timed out"));
    }
}

#[cfg(test)]
mod retry_tests {
    use std::time::Duration;
    use matrix_bridge_wechat::util::retry::{BackoffConfig, ExponentialBackoff, Backoff};
    
    #[test]
    fn test_exponential_backoff() {
        let config = BackoffConfig {
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            multiplier: 2.0,
            max_retries: 5,
            jitter: false,
        };
        
        let mut backoff = ExponentialBackoff::new(config);
        
        let d1 = backoff.next_delay().unwrap();
        assert!(d1 >= Duration::from_millis(100));
        
        let d2 = backoff.next_delay().unwrap();
        assert!(d2 >= Duration::from_millis(100));
        
        assert!(backoff.retry_count() == 2);
    }
    
    #[test]
    fn test_backoff_exhaustion() {
        let config = BackoffConfig {
            initial_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(100),
            multiplier: 2.0,
            max_retries: 3,
            jitter: false,
        };
        
        let mut backoff = ExponentialBackoff::new(config);
        
        backoff.next_delay();
        backoff.next_delay();
        backoff.next_delay();
        
        assert!(backoff.is_exhausted());
        assert!(backoff.next_delay().is_none());
    }
    
    #[test]
    fn test_backoff_reset() {
        let config = BackoffConfig {
            initial_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(100),
            multiplier: 2.0,
            max_retries: 3,
            jitter: false,
        };
        
        let mut backoff = ExponentialBackoff::new(config);
        
        backoff.next_delay();
        backoff.next_delay();
        backoff.reset();
        
        assert!(backoff.retry_count() == 0);
        assert!(!backoff.is_exhausted());
    }
}

#[cfg(test)]
mod cache_tests {
    use std::time::Duration;
    use matrix_bridge_wechat::util::perf::Cache;
    
    #[tokio::test]
    async fn test_cache_basic() {
        let cache: Cache<String, String> = Cache::new(100);
        
        cache.insert("key1".to_string(), "value1".to_string()).await;
        
        let value = cache.get(&"key1".to_string()).await;
        assert!(value.is_some());
        assert_eq!(value.unwrap(), "value1");
    }
    
    #[tokio::test]
    async fn test_cache_missing() {
        let cache: Cache<String, String> = Cache::new(100);
        
        let value = cache.get(&"missing".to_string()).await;
        assert!(value.is_none());
    }
    
    #[tokio::test]
    async fn test_cache_removal() {
        let cache: Cache<String, String> = Cache::new(100);
        
        cache.insert("key1".to_string(), "value1".to_string()).await;
        cache.remove(&"key1".to_string()).await;
        
        let value = cache.get(&"key1".to_string()).await;
        assert!(value.is_none());
    }
    
    #[tokio::test]
    async fn test_cache_size_limit() {
        let cache: Cache<String, String> = Cache::new(3);
        
        cache.insert("key1".to_string(), "value1".to_string()).await;
        cache.insert("key2".to_string(), "value2".to_string()).await;
        cache.insert("key3".to_string(), "value3".to_string()).await;
        cache.insert("key4".to_string(), "value4".to_string()).await;
        
        let len = cache.len().await;
        assert!(len <= 3);
    }
}

#[cfg(test)]
mod queue_tests {
    use matrix_bridge_wechat::util::perf::{MessageQueue, QueueMessage};
    
    #[tokio::test]
    async fn test_queue_push_pop() {
        let queue: MessageQueue<String> = MessageQueue::new(100, 10);
        
        let msg = QueueMessage::new("msg1", "hello".to_string());
        queue.push(msg).await.unwrap();
        
        let popped = queue.pop().await;
        assert!(popped.is_some());
        assert_eq!(popped.unwrap().data, "hello");
    }
    
    #[tokio::test]
    async fn test_queue_order() {
        let queue: MessageQueue<String> = MessageQueue::new(100, 10);
        
        queue.push(QueueMessage::new("1", "first".to_string())).await.unwrap();
        queue.push(QueueMessage::new("2", "second".to_string())).await.unwrap();
        
        let first = queue.pop().await.unwrap();
        let second = queue.pop().await.unwrap();
        
        assert_eq!(first.data, "first");
        assert_eq!(second.data, "second");
    }
    
    #[tokio::test]
    async fn test_queue_full() {
        let queue: MessageQueue<String> = MessageQueue::new(2, 10);
        
        queue.push(QueueMessage::new("1", "first".to_string())).await.unwrap();
        queue.push(QueueMessage::new("2", "second".to_string())).await.unwrap();
        
        let result = queue.push(QueueMessage::new("3", "third".to_string())).await;
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod metrics_tests {
    use matrix_bridge_wechat::metrics::Metrics;
    
    #[tokio::test]
    async fn test_counter() {
        let metrics = Metrics::new();
        
        assert_eq!(metrics.messages_bridged.get().await, 0);
        
        metrics.messages_bridged.inc().await;
        assert_eq!(metrics.messages_bridged.get().await, 1);
        
        metrics.messages_bridged.inc_by(5).await;
        assert_eq!(metrics.messages_bridged.get().await, 6);
    }
    
    #[tokio::test]
    async fn test_gauge() {
        let metrics = Metrics::new();
        
        assert_eq!(metrics.active_users.get().await, 0.0);
        
        metrics.active_users.set(10.0).await;
        assert_eq!(metrics.active_users.get().await, 10.0);
        
        metrics.active_users.inc().await;
        assert_eq!(metrics.active_users.get().await, 11.0);
        
        metrics.active_users.dec().await;
        assert_eq!(metrics.active_users.get().await, 10.0);
    }
    
    #[tokio::test]
    async fn test_prometheus_output() {
        let metrics = Metrics::new();
        
        metrics.messages_bridged.inc().await;
        metrics.active_users.set(5.0).await;
        
        let output = metrics.to_prometheus().await;
        
        assert!(output.contains("bridge_messages_bridged 1"));
        assert!(output.contains("bridge_active_users 5"));
    }
}
