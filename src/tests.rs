use super::*;

#[test]
fn test_parse_xm_header_first() {
    let data_bytes = include_bytes!("test_xms/test_w_mpt_ext.xm");

    let (input, header) = parse_xm_header(data_bytes).unwrap();
    let (input, pattern_order_table_raw) =
        parse_xm_pattern_order_table_raw(input, header.0.song_length as usize, header.3 as usize)
            .unwrap();

    let (input, patterns) = nom::multi::count(
        parse_xm_pattern(header.0.channels_num),
        header.0.patterns_num as usize,
    )(input)
    .unwrap();

    let (input, instrument) = parse_xm_instrument(input).unwrap();

    println!("{:#?}", header);
    println!("{:?}", pattern_order_table_raw);

    println!("{:#?}", instrument);

    // for pattern in patterns.iter() {
    //     println!("{}", pattern.1);
    //     println!();
    // }
}
