use std::convert::Infallible;
use std::error::Error as StdError;
use std::io;
use std::net::{Ipv6Addr, SocketAddr};
use std::time::{Duration, Instant};

use http_body_util::Empty;
use hyper::body::{Bytes, Incoming};
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response, StatusCode};
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

const CONNECTION_TIMEOUT: Duration = Duration::from_secs(5);
const ACCEPT_ERROR_BACKOFF: Duration = Duration::from_millis(100);
const ACCEPT_ERROR_LOG_INTERVAL: Duration = Duration::from_secs(10);

#[tokio::main]
async fn main() -> Result<(), Box<dyn StdError + Send + Sync>> {
    // Linux 默认开启双栈（IPV6_V6ONLY=0），绑定 [::]:80 可同时监听 IPv4 与 IPv6
    let addr = SocketAddr::from((Ipv6Addr::UNSPECIFIED, 80));
    let listener = TcpListener::bind(addr).await?;

    let mut next_accept_error_log = Instant::now();
    let mut unreported_accept_errors = 0u64;

    // 持续接收客户端连接并并发处理
    loop {
        let (stream, _) = match listener.accept().await {
            Ok(conn) => conn,
            // Linux accept(2) 会将已挂起的单连接网络异常直接返回，忽略并继续接收即可
            Err(err) if is_transient_accept_error(&err) => continue,
            // 文件描述符耗尽（EMFILE/ENFILE）等系统资源类错误，记录并短暂退避，避免进程退出或 CPU 空转
            Err(err) => {
                unreported_accept_errors += 1;
                let now = Instant::now();
                if now >= next_accept_error_log {
                    eprintln!("Accept error: {err} ({unreported_accept_errors} since last report)");
                    unreported_accept_errors = 0;
                    next_accept_error_log = now + ACCEPT_ERROR_LOG_INTERVAL;
                }
                tokio::time::sleep(ACCEPT_ERROR_BACKOFF).await;
                continue;
            }
        };

        // 禁用 Nagle 算法，降低短小 204 响应的发送延迟
        let _ = stream.set_nodelay(true);

        // 将 Tokio TcpStream 适配为 hyper 所需的异步 I/O 接口
        let io = TokioIo::new(stream);

        // 为每个连接派生独立异步任务，配合短超时与禁用 Keep-Alive 快速释放资源
        tokio::task::spawn(async move {
            let conn = http1::Builder::new()
                .keep_alive(false)
                .serve_connection(io, service_fn(generate_204));

            match tokio::time::timeout(CONNECTION_TIMEOUT, conn).await {
                // 正常完成或客户端超时均无需输出日志
                Ok(Ok(())) | Err(_) => {}
                // 忽略公网常见的客户端断连、端口扫描及非法 HTTP 报文解析错误
                Ok(Err(err)) if is_ignorable_conn_error(&err) => {}
                Ok(Err(err)) => eprintln!("Unexpected connection error: {err}"),
            }
        });
    }
}

/// 判断 `accept` 错误是否为单个客户端连接引发的瞬态网络异常（参考 Linux `accept(2)`）。
fn is_transient_accept_error(err: &io::Error) -> bool {
    matches!(
        err.kind(),
        io::ErrorKind::ConnectionRefused
            | io::ErrorKind::ConnectionAborted
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::HostUnreachable
            | io::ErrorKind::NetworkUnreachable
            | io::ErrorKind::NetworkDown
            | io::ErrorKind::Interrupted
            | io::ErrorKind::WouldBlock
    )
}

/// 判断连接服务错误是否为常规客户端 I/O 中断或非法报文解析错误。
fn is_ignorable_conn_error(err: &hyper::Error) -> bool {
    if err.is_parse()
        || err.is_parse_too_large()
        || err.is_incomplete_message()
        || err.is_canceled()
        || err.is_closed()
        || err.is_timeout()
    {
        return true;
    }

    let mut source = err.source();
    while let Some(src) = source {
        if src.is::<io::Error>() {
            return true;
        }
        source = src.source();
    }

    false
}

/// 统一返回无响应体的 `204 No Content`。
async fn generate_204(_: Request<Incoming>) -> Result<Response<Empty<Bytes>>, Infallible> {
    let mut resp = Response::new(Empty::new());
    *resp.status_mut() = StatusCode::NO_CONTENT;
    Ok(resp)
}
