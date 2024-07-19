use super::*;

#[test]
fn test_parse_xm_header_first() {
    let data_bytes = include_bytes!("test_xms/agree_with_me.xm");

    let (input, header) = header::parse(data_bytes).unwrap();
    let (input, pattern_order_table_raw) =
        pattern::parse_order_table_raw(input, header.0.song_length as usize, header.3 as usize)
            .unwrap();

    let (input, patterns) = nom::multi::count(
        pattern::parse(header.0.channels_num),
        header.0.patterns_num as usize,
    )(input)
    .unwrap();

    // let (input, instrument) = instrument::parse_h(input).unwrap();

    println!("{:#?}", header);
    println!("{:?}", pattern_order_table_raw);

    // println!("{:#?}", instrument);

    // for pattern in patterns.iter() {
    //     println!("{}", pattern.1);
    //     println!();
    // }
}
