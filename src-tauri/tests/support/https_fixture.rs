//! TLS material and a certificate challenge callback for the native fixture.
use block2::Block;
use objc2::{
    msg_send,
    runtime::{AnyClass, AnyObject, Sel},
    sel,
};
use objc2_foundation::NSString;
use rustls::pki_types::{CertificateDer, PrivatePkcs8KeyDer};
use std::{
    ffi::c_void,
    process::Command,
    sync::{
        Arc, OnceLock,
        atomic::{AtomicBool, Ordering},
    },
};

struct Pin {
    port: u16,
    certificate: Vec<u8>,
}

static PIN: OnceLock<Pin> = OnceLock::new();
pub(super) static ACCEPTED: AtomicBool = AtomicBool::new(false);
pub(super) static REJECTED: AtomicBool = AtomicBool::new(false);

pub(super) fn server_config(port: u16, wrong_pin: bool) -> Arc<rustls::ServerConfig> {
    let directory = tempfile::tempdir().unwrap();
    let openssl = |arguments: &[&str]| {
        let output = Command::new("/usr/bin/openssl")
            .args(arguments)
            .current_dir(directory.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "openssl failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    openssl(&[
        "req",
        "-x509",
        "-newkey",
        "rsa:2048",
        "-nodes",
        "-days",
        "1",
        "-subj",
        "/CN=localhost",
        "-addext",
        "subjectAltName=DNS:localhost",
        "-keyout",
        "key.pem",
        "-out",
        "cert.pem",
    ]);
    openssl(&[
        "x509", "-in", "cert.pem", "-outform", "DER", "-out", "cert.der",
    ]);
    openssl(&[
        "pkcs8", "-topk8", "-nocrypt", "-in", "key.pem", "-outform", "DER", "-out", "key.der",
    ]);
    let certificate = std::fs::read(directory.path().join("cert.der")).unwrap();
    let key = std::fs::read(directory.path().join("key.der")).unwrap();
    let mut pinned_der = certificate.clone();
    if wrong_pin {
        pinned_der[0] ^= 1;
    }
    assert!(
        PIN.set(Pin {
            port,
            certificate: pinned_der
        })
        .is_ok()
    );
    Arc::new(
        rustls::ServerConfig::builder_with_provider(Arc::new(
            rustls::crypto::ring::default_provider(),
        ))
        .with_safe_default_protocol_versions()
        .unwrap()
        .with_no_client_auth()
        .with_single_cert(
            vec![CertificateDer::from(certificate)],
            PrivatePkcs8KeyDer::from(key).into(),
        )
        .unwrap(),
    )
}

type ChallengeResult = Block<dyn Fn(isize, *mut AnyObject)>;

#[link(name = "Security", kind = "framework")]
unsafe extern "C" {
    fn SecTrustGetCertificateAtIndex(trust: *const c_void, index: isize) -> *const c_void;
    fn SecCertificateCopyData(certificate: *const c_void) -> *const c_void;
}

#[link(name = "CoreFoundation", kind = "framework")]
unsafe extern "C" {
    fn CFDataGetLength(data: *const c_void) -> isize;
    fn CFDataGetBytePtr(data: *const c_void) -> *const u8;
    fn CFRelease(value: *const c_void);
}

#[link(name = "objc")]
unsafe extern "C" {
    fn class_addMethod(
        class: &AnyClass,
        selector: Sel,
        implementation: *const c_void,
        encoding: *const std::ffi::c_char,
    ) -> bool;
}

unsafe extern "C-unwind" fn handle_challenge(
    _: &AnyObject,
    _: Sel,
    _: &AnyObject,
    challenge: &AnyObject,
    completion: &ChallengeResult,
) {
    let pin = PIN.get().unwrap();
    unsafe {
        let protection: Option<objc2::rc::Retained<AnyObject>> =
            msg_send![challenge, protectionSpace];
        if let Some(protection) = protection {
            let host: Option<objc2::rc::Retained<NSString>> = msg_send![&*protection, host];
            let method: Option<objc2::rc::Retained<NSString>> =
                msg_send![&*protection, authenticationMethod];
            let port: isize = msg_send![&*protection, port];
            if host.as_deref() == Some(&*NSString::from_str("localhost"))
                && method.as_deref()
                    == Some(&*NSString::from_str("NSURLAuthenticationMethodServerTrust"))
                && port == pin.port as isize
            {
                let trust: *const c_void = msg_send![&*protection, serverTrust];
                if !trust.is_null() {
                    let certificate = SecTrustGetCertificateAtIndex(trust, 0);
                    if !certificate.is_null() {
                        let data = SecCertificateCopyData(certificate);
                        if !data.is_null() {
                            let bytes = std::slice::from_raw_parts(
                                CFDataGetBytePtr(data),
                                CFDataGetLength(data) as usize,
                            );
                            let matches = bytes == pin.certificate;
                            CFRelease(data);
                            if matches {
                                ACCEPTED.store(true, Ordering::SeqCst);
                                eprintln!("fixture TLS: accepted the pinned certificate");
                                let credential: *mut AnyObject = msg_send![objc2::class!(NSURLCredential), credentialForTrust: trust];
                                completion.call((0, credential)); // NSURLSessionAuthChallengeUseCredential
                                return;
                            }
                        }
                    }
                }
            }
        }
        eprintln!("fixture TLS: rejected the authentication challenge");
        completion.call((2, std::ptr::null_mut())); // NSURLSessionAuthChallengeCancelAuthenticationChallenge
        REJECTED.store(true, Ordering::SeqCst);
    }
}

pub(super) fn install_challenge_callback(webview: &AnyObject) {
    // Preserve Wry's other delegate methods, including its download routing.
    // This method exists only in the test executable and checks the exact
    // fixture port, host, and DER bytes before supplying a credential.
    unsafe {
        let delegate: Option<objc2::rc::Retained<AnyObject>> =
            msg_send![webview, navigationDelegate];
        let delegate = delegate.expect("Wry navigation delegate");
        assert!(class_addMethod(
            delegate.class(),
            sel!(webView:didReceiveAuthenticationChallenge:completionHandler:),
            handle_challenge as *const c_void,
            c"v@:@@@?".as_ptr(),
        ));
        // WebKit records the supported selectors when the delegate is set.
        let _: () = msg_send![webview, setNavigationDelegate: std::ptr::null::<AnyObject>()];
        let _: () = msg_send![webview, setNavigationDelegate: &*delegate];
    }
}
