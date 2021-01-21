//! Client for querying DNS servers.
use std::net::{Ipv4Addr, SocketAddr};
use std::str::FromStr;
use tokio::net::UdpSocket;
use tokio::runtime::Runtime;

use trust_dns_client::udp::UdpClientStream;
use trust_dns_client::client::{Client, AsyncClient, ClientHandle};
use trust_dns_client::rr::{DNSClass, Name, RData, Record, RecordType};
use trust_dns_client::op::{DnsResponse, ResponseCode};
use trust_dns_client::rr::rdata::key::KEY;

use crate::{Error, Result};

// ([8,8,8,8], 53).into()
// DNSClass::IN,
// RecordType::A

/// Query a DNS server and return the response.
pub async fn query(
    // Address to the DNS resolver, eg: `([8,8,8,8], 53).into()` 
    name_server: SocketAddr,
    // Fully qualified domain name including trailing period
    fqdn: &str,
    // The class of request, typically `DNSClass::IN` (internet)
    class: DNSClass,
    // The DNS record type, eg: `A`, `AAAA` or `NS`.
    record_type: RecordType,
    ) -> Result<DnsResponse> {

    let stream = UdpClientStream::<UdpSocket>::new(name_server);

    // Create the UDP socket connection
    let (mut client, bg) = AsyncClient::connect(stream).await?;

    // Spawn the DNS background task
    tokio::spawn(bg);

    // Create a query future
    let query = client.query(
        Name::from_str(fqdn)?,
        class,
        record_type,
    );

    // Get the response
    let response = query.await?;

    /*
    // validate it's what we expected
    if let &RData::A(addr) = response.answers()[0].rdata() {
        assert_eq!(addr, Ipv4Addr::new(93, 184, 216, 34));
    }
    */

    Ok(response)
}
