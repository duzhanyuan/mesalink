/* ssl.rs
 *                            _ _       _    
 *                           | (_)     | |   
 *  _ __ ___   ___  ___  __ _| |_ _ __ | | __
 * | '_ ` _ \ / _ \/ __|/ _` | | | '_ \| |/ /
 * | | | | | |  __/\__ \ (_| | | | | | |   < 
 * |_| |_| |_|\___||___/\__,_|_|_|_| |_|_|\_\
 *
 * Copyright (C) 2017 Baidu USA.
 *
 * This file is part of Mesalink.
 */

#![allow(non_snake_case)]

use std::sync::Arc;
use std::net::TcpStream;
use std::io::{Read, Write};
use std::ffi::CStr;
use std::os::unix::io::FromRawFd;
use std::slice;
use std::ptr;
use libc::{c_uchar, c_char, c_int};

use rustls::{ClientConfig, ProtocolVersion};
use rustls::{ClientSession, Stream};
use webpki::DNSNameRef;
use webpki_roots::TLS_SERVER_ROOTS;

const MAGIC: u32 = 0xc0d4c5a9;

#[repr(C)]
pub struct MESALINK_METHOD {
    magic: u32,
    tls_version: ProtocolVersion,
}

#[repr(C)]
pub struct MESALINK_CTX {
    magic: u32,
    config: Arc<ClientConfig>,
}

#[repr(C)]
pub struct MESALINK_SSL<'a> {
    magic: u32,
    context: &'a mut MESALINK_CTX,
    session: Option<ClientSession>,
    socket: Option<TcpStream>,
    stream: Option<Stream<'a, ClientSession, TcpStream>>,
}

pub enum SslConstants {
    SslFailure = 0,
    SslSuccess = 1,
}

#[no_mangle]
pub extern "C" fn mesalink_SSLv3_client_method() -> *mut MESALINK_METHOD {
    let p: *mut MESALINK_METHOD = ptr::null_mut();
    p
}

#[no_mangle]
pub extern "C" fn mesalink_TLSv1_client_method() -> *mut MESALINK_METHOD {
    let p: *mut MESALINK_METHOD = ptr::null_mut();
    p
}

#[no_mangle]
pub extern "C" fn mesalink_TLSv1_1_client_method() -> *mut MESALINK_METHOD {
    let p: *mut MESALINK_METHOD = ptr::null_mut();
    p
}

#[no_mangle]
pub extern "C" fn mesalink_TLSv1_2_client_method() -> *mut MESALINK_METHOD {
    let method = MESALINK_METHOD {
        magic: MAGIC,
        tls_version: ProtocolVersion::TLSv1_2,
    };
    Box::into_raw(Box::new(method))
}

#[no_mangle]
pub extern "C" fn mesalink_TLSv1_3_client_method() -> *mut MESALINK_METHOD {
    let method = MESALINK_METHOD {
        magic: MAGIC,
        tls_version: ProtocolVersion::TLSv1_3,
    };
    Box::into_raw(Box::new(method))
}

#[no_mangle]
pub extern "C" fn mesalink_CTX_new(method_ptr: *mut MESALINK_METHOD) -> *mut MESALINK_CTX {
    let method = unsafe {
        assert!(!method_ptr.is_null(), "MESALINK_METHOD pointer is null");
        &*method_ptr
    };
    assert!(method.magic == MAGIC, "Corrupted MESALINK_METHOD pointer");
    let mut client_config = ClientConfig::new();
    client_config.versions = vec![method.tls_version];
    client_config.root_store.add_server_trust_anchors(&TLS_SERVER_ROOTS);
    let context = MESALINK_CTX {
        magic: MAGIC,
        config: Arc::new(client_config),
    };
    Box::into_raw(Box::new(context))
}

#[no_mangle]
pub extern "C" fn mesalink_SSL_new<'a>(ctx_ptr: *mut MESALINK_CTX) -> *mut MESALINK_SSL<'a> {
    let ctx = unsafe {
        assert!(!ctx_ptr.is_null(), "MESALINK_CTX pointer is null");
        &mut *ctx_ptr
    };
    assert!(ctx.magic == MAGIC, "Corrupted MESALINK_CTX pointer");
    let ssl = MESALINK_SSL {
        magic: MAGIC,
        context: ctx,
        session: None,
        socket: None,
        stream: None,
    };
    Box::into_raw(Box::new(ssl))
}

#[no_mangle]
pub extern "C" fn mesalink_SSL_set_tlsext_host_name(ssl_ptr: *mut MESALINK_SSL, hostname_ptr: *const c_char) -> c_int {
    let ssl = unsafe {
        assert!(!ssl_ptr.is_null(), "MESALINK_CTX pointer is null");
        &mut *ssl_ptr
    };
    assert!(ssl.magic == MAGIC, "Corrupted MESALINK_SSL pointer");
    let hostname = unsafe {
        assert!(!hostname_ptr.is_null(), "Hostname is null");
        CStr::from_ptr(hostname_ptr)
    };
    if let Ok(hostname_str) = hostname.to_str() {
        match DNSNameRef::try_from_ascii_str(hostname_str) {
            Ok(dnsname) => {
                ssl.session = Some(ClientSession::new(&ssl.context.config, dnsname));
                SslConstants::SslSuccess as c_int
            },
            Err(_) => {
                SslConstants::SslFailure as c_int
            }
        }
    } else {
        SslConstants::SslFailure as c_int
    }
}

#[no_mangle]
pub extern "C" fn mesalink_SSL_set_fd(ssl_ptr: *mut MESALINK_SSL, fd: c_int) -> c_int {
    let ssl = unsafe {
        assert!(!ssl_ptr.is_null(), "MESALINK_CTX pointer is null");
        &mut *ssl_ptr
    };
    assert!(ssl.magic == MAGIC, "Corrupted MESALINK_SSL pointer");
    let socket = unsafe { TcpStream::from_raw_fd(fd) };
    ssl.socket = Some(socket);
    let stream = Stream::new(ssl.session.as_mut().unwrap(), ssl.socket.as_mut().unwrap());
    ssl.stream = Some(stream);
    SslConstants::SslSuccess as c_int
}

#[no_mangle]
pub extern "C" fn mesalink_SSL_connect(ssl_ptr: *mut MESALINK_SSL) -> c_int {
    let ssl = unsafe {
        assert!(!ssl_ptr.is_null(), "MESALINK_SSL pointer is null");
        &mut *ssl_ptr
    };
    assert!(ssl.magic == MAGIC, "Corrupted MESALINK_SSL pointer");
    match ssl.stream {
        Some(_) => SslConstants::SslSuccess as c_int,
        None => SslConstants::SslFailure as c_int,
    }
}

#[no_mangle]
pub extern "C" fn mesalink_SSL_read(ssl_ptr: *mut MESALINK_SSL, buf_ptr: *mut c_uchar, buf_len: c_int) -> c_int {
    let ssl = unsafe {
        assert!(!ssl_ptr.is_null(), "MESALINK_SSL pointer is null");
        &mut *ssl_ptr
    };
    assert!(ssl.magic == MAGIC, "Corrupted MESALINK_SSL pointer");
    let mut buf = unsafe { slice::from_raw_parts_mut(buf_ptr, buf_len as usize) };
    let stream = ssl.stream.as_mut().unwrap();
    match stream.read(&mut buf) {
        Ok(count) => count as c_int,
        Err(_) => SslConstants::SslFailure as c_int,
    }
}

#[no_mangle]
pub extern "C" fn mesalink_SSL_write(ssl_ptr: *mut MESALINK_SSL, buf_ptr: *const c_uchar, buf_len: c_int) -> c_int {
    let ssl = unsafe {
        assert!(!ssl_ptr.is_null(), "MESALINK_SSL pointer is null");
        &mut *ssl_ptr
    };
    assert!(ssl.magic == MAGIC, "Corrupted MESALINK_SSL pointer");
    let buf = unsafe { slice::from_raw_parts(buf_ptr, buf_len as usize) };
    let stream = ssl.stream.as_mut().unwrap();
    match stream.write(buf) {
        Ok(count) => count as c_int,
        Err(_) => SslConstants::SslFailure as c_int,
    }
}

#[no_mangle]
pub extern "C" fn mesalink_CTX_free(ctx_ptr: *mut MESALINK_CTX) {
    let ctx = unsafe {
        assert!(!ctx_ptr.is_null(), "MESALINK_CTX pointer is null");
        &mut *ctx_ptr
    };
    assert!(ctx.magic == MAGIC, "Corrupted MESALINK_CTX pointer");
    unsafe {
        let _to_be_dropped = Box::from_raw(ctx_ptr);
    }
}

#[no_mangle]
pub extern "C" fn mesalink_SSL_free(ptr: *mut MESALINK_SSL) {
    let ssl = unsafe {
        assert!(!ptr.is_null(), "MESALINK_SSL pointer is null");
        &mut *ptr
    };
    assert!(ssl.magic == MAGIC, "Corrupted MESALINK_SSL pointer");
    unsafe {
        let _to_be_dropped = Box::from_raw(ptr);
    }
}