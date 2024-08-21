
use crate::deps;

use deps::x509_parser;
use x509_parser::pem::Pem;

pub fn parse_certs(certs: &[u8]) -> Vec<Vec<u8>> {
    let pem_iter = Pem::iter_from_buffer(certs);
    let mut der_vec = Vec::new();
    for pem in pem_iter {
        match pem {
            Ok(pem) => {
                der_vec.push(pem.contents);
            }
            Err(e) => {
                log::error!("Error parsing PEM: {:?}", e);
            }
        }
    }
    der_vec
}

pub fn parse_cert(cert: &[u8]) -> Option<Vec<u8>> {
    let pem_iter = Pem::iter_from_buffer(cert);
    for pem in pem_iter {
        match pem {
            Ok(pem) => {
                return Some(pem.contents);
            }
            Err(e) => {
                log::error!("Error parsing PEM: {:?}", e);
            }
        }
    }
    None
}
