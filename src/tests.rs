use super::*;

#[test]
fn test_parse_xm_header_first() {
    let (_input, format) = parse(include_bytes!("test_xms/surfonasinewave.xm")).unwrap();

    println!("{:?}", format.pattern_order_table);
    println!("{:#?}", format.instruments.len());
    println!("{}", format.patterns[0].1);
}
