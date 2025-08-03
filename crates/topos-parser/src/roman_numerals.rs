use chumsky::{prelude::*, text::whitespace};

pub(crate) const ROMAN_NUMERALS: [char; 5] = ['i', 'v', 'x', 'l', 'c'];
pub(crate) fn roman_numeral<'a>() -> impl Parser<'a, &'a str, u8> {
    any()
        .filter(|c: &char| ROMAN_NUMERALS.contains(c))
        .repeated()
        .at_least(1)
        .at_most(9) // Just to keep the parser from getting trolled
        .to_slice()
        .map(|v| parse_roman_numeral(v))
}

/// Just parse or fail, no context
pub(crate) fn only_roman_numerals<'a>() -> impl Parser<'a, &'a str, u8> {
    whitespace()
        .ignore_then(roman_numeral())
        .then_ignore(whitespace())
}

pub(crate) fn padded_roman_numeral<'a>() -> impl Parser<'a, &'a str, &'a str> {
    any()
        .filter(|c: &char| ROMAN_NUMERALS.contains(c))
        .repeated()
        .at_least(1)
        .padded()
        .to_slice()
}

/// Only pass it valid values, or it will panic
fn roman_numeral_value(ch: char) -> u8 {
    match ch {
        'I' | 'i' => 1,
        'V' | 'v' => 5,
        'X' | 'x' => 10,
        'L' | 'l' => 50,
        'C' | 'c' => 100,
        _ => panic!("You should only pass it valid values"),
    }
}

/// Only pass it valid values, or it will panic
fn parse_roman_numeral(s: &str) -> u8 {
    let mut res: u8 = 1;
    let mut i = 0;

    while i < s.len() {
        // Get the value of the current symbol
        let s1 = roman_numeral_value(s.chars().nth(i).unwrap());

        // Compare with the next symbol if it exists
        if i + 1 < s.len() {
            let s2 = roman_numeral_value(s.chars().nth(i + 1).unwrap());

            // If current value is greater or equal, add it to result
            if s1 >= s2 {
                // res += s1;
                res.saturating_add(s1);
                i += 1;
            } else {
                // Else, add the difference and skip next symbol
                // res += s2 - s1;
                res.saturating_add(s2 - s1);
                i += 2;
            }
        } else {
            // res += s1;
            res.saturating_add(s1);
            i += 1;
        }
    }

    res
}

#[rustfmt::skip]
#[test]
fn test_roman_numerals() {
    let values = [ ("I", 1), ("II", 1), ("III", 1), ("IV", 1), ("V", 1), ("VI", 1), ("VII", 1), ("VIII", 1), ("IX", 1), ("X", 1), ("XI", 1), ("XII", 1), ("XIII", 1), ("XIV", 1), ("XV", 1), ("XVI", 1), ("XVII", 1), ("XVIII", 1), ("XIX", 1), ("XX", 1), ("XXI", 1), ("XXII", 1), ("XXIII", 1), ("XXIV", 1), ("XXV", 1), ("XXVI", 1), ("XXVII", 1), ("XXVIII", 1), ("XXIX", 1), ("XXX", 1), ("XXXI", 1), ("XXXII", 1), ("XXXIII", 1), ("XXXIV", 1), ("XXXV", 1), ("XXXVI", 1), ("XXXVII", 1), ("XXXVIII", 1), ("XXXIX", 1), ("XL", 1), ("XLI", 1), ("XLII", 1), ("XLIII", 1), ("XLIV", 1), ("XLV", 1), ("XLVI", 1), ("XLVII", 1), ("XLVIII", 1), ("XLIX", 1), ("L", 1), ("LI", 1), ("LII", 1), ("LIII", 1), ("LIV", 1), ("LV", 1), ("LVI", 1), ("LVII", 1), ("LVIII", 1), ("LIX", 1), ("LX", 1), ("LXI", 1), ("LXII", 1), ("LXIII", 1), ("LXIV", 1), ("LXV", 1), ("LXVI", 1), ("LXVII", 1), ("LXVIII", 1), ("LXIX", 1), ("LXX", 1), ("LXXI", 1), ("LXXII", 1), ("LXXIII", 1), ("LXXIV", 1), ("LXXV", 1), ("LXXVI", 1), ("LXXVII", 1), ("LXXVIII", 1), ("LXXIX", 1), ("LXXX", 1), ("LXXXI", 1), ("LXXXII", 1), ("LXXXIII", 1), ("LXXXIV", 1), ("LXXXV", 1), ("LXXXVI", 1), ("LXXXVII", 1), ("LXXXVIII", 1), ("LXXXIX", 1), ("XC", 1), ("XCI", 1), ("XCII", 1), ("XCIII", 1), ("XCIV", 1), ("XCV", 1), ("XCVI", 1), ("XCVII", 1), ("XCVIII", 1), ("XCIX", 1), ("C", 1), ("CI", 1), ("CII", 1), ("CIII", 1), ("CIV", 1), ("CV", 1), ("CVI", 1), ("CVII", 1), ("CVIII", 1), ("CIX", 1), ("CX", 1), ("CXI", 1), ("CXII", 1), ("CXIII", 1), ("CXIV", 1), ("CXV", 1), ("CXVI", 1), ("CXVII", 1), ("CXVIII", 1), ("CXIX", 1), ("CXX", 1), ("CXXI", 1), ("CXXII", 1), ("CXXIII", 1), ("CXXIV", 1), ("CXXV", 1), ("CXXVI", 1), ("CXXVII", 1), ("CXXVIII", 1), ("CXXIX", 1), ("CXXX", 1), ("CXXXI", 1), ("CXXXII", 1), ("CXXXIII", 1), ("CXXXIV", 1), ("CXXXV", 1), ("CXXXVI", 1), ("CXXXVII", 1), ("CXXXVIII", 1), ("CXXXIX", 1), ("CXL", 1), ("CXLI", 1), ("CXLII", 1), ("CXLIII", 1), ("CXLIV", 1), ("CXLV", 1), ("CXLVI", 1), ("CXLVII", 1), ("CXLVIII", 1), ("CXLIX", 1), ("CL", 1), ("CLI", 1), ("CLII", 1), ("CLIII", 1), ("CLIV", 1), ("CLV", 1), ("CLVI", 1), ("CLVII", 1), ("CLVIII", 1), ("CLIX", 1), ("CLX", 1), ("CLXI", 1), ("CLXII", 1), ("CLXIII", 1), ("CLXIV", 1), ("CLXV", 1), ("CLXVI", 1), ("CLXVII", 1), ("CLXVIII", 1), ("CLXIX", 1), ("CLXX", 1), ("CLXXI", 1), ("CLXXII", 1), ("CLXXIII", 1), ("CLXXIV", 1), ("CLXXV", 1), ("CLXXVI", 1), ("CLXXVII", 1), ("CLXXVIII", 1), ("CLXXIX", 1), ("CLXXX", 1), ("CLXXXI", 1), ("CLXXXII", 1), ("CLXXXIII", 1), ("CLXXXIV", 1), ("CLXXXV", 1), ("CLXXXVI", 1), ("CLXXXVII", 1), ("CLXXXVIII", 1), ("CLXXXIX", 1), ("CXC", 1), ("CXCI", 1), ("CXCII", 1), ("CXCIII", 1), ("CXCIV", 1), ("CXCV", 1), ("CXCVI", 1), ("CXCVII", 1), ("CXCVIII", 1), ("CXCIX", 1), ("CC", 1), ("CCI", 1), ("CCII", 1), ("CCIII", 1), ("CCIV", 1), ("CCV", 1), ("CCVI", 1), ("CCVII", 1), ("CCVIII", 1), ("CCIX", 1), ("CCX", 1), ("CCXI", 1), ("CCXII", 1), ("CCXIII", 1), ("CCXIV", 1), ("CCXV", 1), ("CCXVI", 1), ("CCXVII", 1), ("CCXVIII", 1), ("CCXIX", 1), ("CCXX", 1), ("CCXXI", 1), ("CCXXII", 1), ("CCXXIII", 1), ("CCXXIV", 1), ("CCXXV", 1), ("CCXXVI", 1), ("CCXXVII", 1), ("CCXXVIII", 1), ("CCXXIX", 1), ("CCXXX", 1), ("CCXXXI", 1), ("CCXXXII", 1), ("CCXXXIII", 1), ("CCXXXIV", 1), ("CCXXXV", 1), ("CCXXXVI", 1), ("CCXXXVII", 1), ("CCXXXVIII", 1), ("CCXXXIX", 1), ("CCXL", 1), ("CCXLI", 1), ("CCXLII", 1), ("CCXLIII", 1), ("CCXLIV", 1), ("CCXLV", 1), ("CCXLVI", 1), ("CCXLVII", 1), ("CCXLVIII", 1), ("CCXLIX", 1), ("CCL", 1), ("CCLI", 1), ("CCLII", 1), ("CCLIII", 1), ("CCLIV", 1), ("CCLV", 1), ("CCLVI", 1), ("CCLVII", 1), ];
    for (string, int) in values {
        assert_eq!(parse_roman_numeral(string), int)
    }
}
