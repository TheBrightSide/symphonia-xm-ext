// use context::XmPlaybackContext;

use super::*;

#[test]
fn test_parse_xm_header_first() {
    let (_input, format) = parse(include_bytes!("test_xms/test_w_mpt_ext.xm")).unwrap();

    println!("{:?}", format.pattern_order_table);
    println!("{:#?}", format.instruments.len());
    println!("{}", format.patterns[0].1);

    // let mut context = XmPlaybackContext::new(format, 44100);
}
