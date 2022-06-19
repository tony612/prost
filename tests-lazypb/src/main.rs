use std::cell::RefCell;

use bytes::Bytes;
use prost::Message;
use prost_types::Timestamp;
use tests_lazypb::pb;

fn main() {
    let mut person = pb::tutorial::Person::default();
    let mut ts = Timestamp::default();
    ts.seconds = 1654700916;
    person.last_updated = Some(RefCell::new(lazypb::Lazy::Ready(ts)));

    let mut bytes = Vec::new();
    person.encode(&mut bytes).unwrap();
    dbg!(person);

    let p2 = pb::tutorial::Person::decode(&*bytes).unwrap();
    dbg!(bytes);

    let mut bytes2 = Vec::new();
    p2.encode(&mut bytes2).unwrap();
    dbg!(&bytes2);
    dbg!(&p2);
    // dbg!(p2.get_last_updated2().unwrap());
    dbg!(&p2.last_updated);
    dbg!(p2.get_last_updated().unwrap());
    dbg!(&p2.last_updated);

    let mut p3 = pb::tutorial::Person::default();
    p3.last_updated = Some(RefCell::new(lazypb::Lazy::Init));
    p3.merge(&*bytes2).unwrap();
    dbg!(&p3);

    let mut p4 = pb::tutorial::Person::default();
    p4.last_updated = Some(RefCell::new(lazypb::Lazy::Ready(Timestamp::default())));
    p4.merge(&*bytes2).unwrap();
    dbg!(&p4);

    let mut p5 = pb::tutorial::Person::default();
    p5.last_updated = Some(RefCell::new(lazypb::Lazy::Pending(Bytes::from("abcd"))));
    p5.merge(&*bytes2).unwrap();
    dbg!(&p5);
}
