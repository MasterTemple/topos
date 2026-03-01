use topos_lib::{
    error::AnyResult,
    matcher::{location::srt::SRTLocation, matcher::BibleMatcher},
};

#[test]
fn tdp_c1() -> AnyResult<()> {
    let srt = include_str!("./Chapter 1 - The Command of Christ transcript.srt");
    let matcher = BibleMatcher::default();
    let results = matcher.search::<SRTLocation>(srt)?;
    dbg!(results);
    Ok(())
}
