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
/// I didn't write this, but it works
fn parse_roman_numeral(s: &str) -> u8 {
    let mut res: u8 = 0;
    let mut i = 0;

    while i < s.len() {
        // Get the value of the current symbol
        let s1 = roman_numeral_value(s.chars().nth(i).unwrap());

        // Compare with the next symbol if it exists
        if i + 1 < s.len() {
            let s2 = roman_numeral_value(s.chars().nth(i + 1).unwrap());

            // If current value is greater or equal, add it to result
            if s1 >= s2 {
                res = res.saturating_add(s1);
                i += 1;
            } else {
                // Else, add the difference and skip next symbol
                res = res.saturating_add(s2 - s1);
                i += 2;
            }
        } else {
            res = res.saturating_add(s1);
            i += 1;
        }
    }

    res
}

#[cfg(test)]
mod tests {
    use super::*;

    #[rustfmt::skip]
    #[test]
    fn test_roman_numerals() {
        let values = [ ("I", 1), ("II", 2), ("III", 3), ("IV", 4), ("V", 5), ("VI", 6), ("VII", 7), ("VIII", 8), ("IX", 9), ("X", 10), ("XI", 11), ("XII", 12), ("XIII", 13), ("XIV", 14), ("XV", 15), ("XVI", 16), ("XVII", 17), ("XVIII", 18), ("XIX", 19), ("XX", 20), ("XXI", 21), ("XXII", 22), ("XXIII", 23), ("XXIV", 24), ("XXV", 25), ("XXVI", 26), ("XXVII", 27), ("XXVIII", 28), ("XXIX", 29), ("XXX", 30), ("XXXI", 31), ("XXXII", 32), ("XXXIII", 33), ("XXXIV", 34), ("XXXV", 35), ("XXXVI", 36), ("XXXVII", 37), ("XXXVIII", 38), ("XXXIX", 39), ("XL", 40), ("XLI", 41), ("XLII", 42), ("XLIII", 43), ("XLIV", 44), ("XLV", 45), ("XLVI", 46), ("XLVII", 47), ("XLVIII", 48), ("XLIX", 49), ("L", 50), ("LI", 51), ("LII", 52), ("LIII", 53), ("LIV", 54), ("LV", 55), ("LVI", 56), ("LVII", 57), ("LVIII", 58), ("LIX", 59), ("LX", 60), ("LXI", 61), ("LXII", 62), ("LXIII", 63), ("LXIV", 64), ("LXV", 65), ("LXVI", 66), ("LXVII", 67), ("LXVIII", 68), ("LXIX", 69), ("LXX", 70), ("LXXI", 71), ("LXXII", 72), ("LXXIII", 73), ("LXXIV", 74), ("LXXV", 75), ("LXXVI", 76), ("LXXVII", 77), ("LXXVIII", 78), ("LXXIX", 79), ("LXXX", 80), ("LXXXI", 81), ("LXXXII", 82), ("LXXXIII", 83), ("LXXXIV", 84), ("LXXXV", 85), ("LXXXVI", 86), ("LXXXVII", 87), ("LXXXVIII", 88), ("LXXXIX", 89), ("XC", 90), ("XCI", 91), ("XCII", 92), ("XCIII", 93), ("XCIV", 94), ("XCV", 95), ("XCVI", 96), ("XCVII", 97), ("XCVIII", 98), ("XCIX", 99), ("C", 100), ("CI", 101), ("CII", 102), ("CIII", 103), ("CIV", 104), ("CV", 105), ("CVI", 106), ("CVII", 107), ("CVIII", 108), ("CIX", 109), ("CX", 110), ("CXI", 111), ("CXII", 112), ("CXIII", 113), ("CXIV", 114), ("CXV", 115), ("CXVI", 116), ("CXVII", 117), ("CXVIII", 118), ("CXIX", 119), ("CXX", 120), ("CXXI", 121), ("CXXII", 122), ("CXXIII", 123), ("CXXIV", 124), ("CXXV", 125), ("CXXVI", 126), ("CXXVII", 127), ("CXXVIII", 128), ("CXXIX", 129), ("CXXX", 130), ("CXXXI", 131), ("CXXXII", 132), ("CXXXIII", 133), ("CXXXIV", 134), ("CXXXV", 135), ("CXXXVI", 136), ("CXXXVII", 137), ("CXXXVIII", 138), ("CXXXIX", 139), ("CXL", 140), ("CXLI", 141), ("CXLII", 142), ("CXLIII", 143), ("CXLIV", 144), ("CXLV", 145), ("CXLVI", 146), ("CXLVII", 147), ("CXLVIII", 148), ("CXLIX", 149), ("CL", 150), ("CLI", 151), ("CLII", 152), ("CLIII", 153), ("CLIV", 154), ("CLV", 155), ("CLVI", 156), ("CLVII", 157), ("CLVIII", 158), ("CLIX", 159), ("CLX", 160), ("CLXI", 161), ("CLXII", 162), ("CLXIII", 163), ("CLXIV", 164), ("CLXV", 165), ("CLXVI", 166), ("CLXVII", 167), ("CLXVIII", 168), ("CLXIX", 169), ("CLXX", 170), ("CLXXI", 171), ("CLXXII", 172), ("CLXXIII", 173), ("CLXXIV", 174), ("CLXXV", 175), ("CLXXVI", 176), ("CLXXVII", 177), ("CLXXVIII", 178), ("CLXXIX", 179), ("CLXXX", 180), ("CLXXXI", 181), ("CLXXXII", 182), ("CLXXXIII", 183), ("CLXXXIV", 184), ("CLXXXV", 185), ("CLXXXVI", 186), ("CLXXXVII", 187), ("CLXXXVIII", 188), ("CLXXXIX", 189), ("CXC", 190), ("CXCI", 191), ("CXCII", 192), ("CXCIII", 193), ("CXCIV", 194), ("CXCV", 195), ("CXCVI", 196), ("CXCVII", 197), ("CXCVIII", 198), ("CXCIX", 199), ("CC", 200), ("CCI", 201), ("CCII", 202), ("CCIII", 203), ("CCIV", 204), ("CCV", 205), ("CCVI", 206), ("CCVII", 207), ("CCVIII", 208), ("CCIX", 209), ("CCX", 210), ("CCXI", 211), ("CCXII", 212), ("CCXIII", 213), ("CCXIV", 214), ("CCXV", 215), ("CCXVI", 216), ("CCXVII", 217), ("CCXVIII", 218), ("CCXIX", 219), ("CCXX", 220), ("CCXXI", 221), ("CCXXII", 222), ("CCXXIII", 223), ("CCXXIV", 224), ("CCXXV", 225), ("CCXXVI", 226), ("CCXXVII", 227), ("CCXXVIII", 228), ("CCXXIX", 229), ("CCXXX", 230), ("CCXXXI", 231), ("CCXXXII", 232), ("CCXXXIII", 233), ("CCXXXIV", 234), ("CCXXXV", 235), ("CCXXXVI", 236), ("CCXXXVII", 237), ("CCXXXVIII", 238), ("CCXXXIX", 239), ("CCXL", 240), ("CCXLI", 241), ("CCXLII", 242), ("CCXLIII", 243), ("CCXLIV", 244), ("CCXLV", 245), ("CCXLVI", 246), ("CCXLVII", 247), ("CCXLVIII", 248), ("CCXLIX", 249), ("CCL", 250), ("CCLI", 251), ("CCLII", 252), ("CCLIII", 253), ("CCLIV", 254), ("CCLV", 255), ("CCLVI", 255), ("CCLVII", 255), ];
        for (string, int) in values {
            assert_eq!(parse_roman_numeral(string), int)
        }
    }
}
